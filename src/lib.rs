#[macro_use]
extern crate tracing;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_modbus::client::{tcp, Context, Reader, Writer};
use anyhow::Result;
use tokio_modbus::{Address, Quantity, Slave, SlaveId};
use tokio::time::timeout;
use crate::consts::{DEFAULT_BACKOFF_BASE_MS, DEFAULT_NETWORK_TIMEOUT_MS, ERROR_INVALID_RESPONSE_HEADER};
use crate::errors::PZEMError;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::RetryIf;
use serde::{Serialize, Deserialize};

pub mod errors;
pub mod consts;

pub type Word = u16;

pub trait PZEMConn: Reader + Writer {}

impl PZEMConn for Context {}

/// A SunSpecConnection holds the address and slave id for the modbus connection, as well as the
/// actual connection object itself as well as the modeldata for all of the exposed models on
/// that connection.
#[derive(Debug, Clone)]
pub struct PZEM {
    pub addr: SocketAddr,
    pub ctx: Arc<Mutex<Box<dyn PZEMConn>>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PZEMData {
    pub volts: f32,
    pub amps: f32,
    pub watts: f32,
    pub watt_hours: f32,
    pub frequency: f32,
    pub power_factor: f32,
    //pub alarm_status: u32
}

impl PZEM {
    pub async fn new(
        socket_addr: String,
    ) -> anyhow::Result<Self> {
        let socket_addr = socket_addr.parse().unwrap();
        let ctx: Context;
        ctx = match tcp::connect(socket_addr).await {
            Ok(ctx) => ctx,
            Err(e) => {
                anyhow::bail!("Can't connect: {e}");
            }
        };
        //let arc_ctx = Arc::new(Mutex::new(ctx));
        Ok(PZEM {
            addr: socket_addr,
            ctx: Arc::new(Mutex::new(Box::new(ctx))),
        })
    }
    pub async fn select_slave(&mut self, slave_id: u8) {
        debug!("Selecting slave {slave_id}");
        let mut ctx = self.ctx.lock().await;
        ctx.set_slave(Slave(slave_id));
    }
    pub async fn set_unit_slave_address(&mut self, addr: SlaveId) -> Result<(), PZEMError> {
        let mut ctx = self.ctx.lock().await;
        match ctx.write_single_register(0x2, addr as u16).await {
            Ok(_) => Ok(()),
            Err(e) => return Err(PZEMError::Misc(e.to_string()))
        }
    }
    pub async fn get_u16(&mut self, slave_id: u8, addr: Address) -> Result<u16, PZEMError> {
        self.select_slave(slave_id).await;
        let data = match self.clone().retry_read_input_registers(addr, 1).await {
            Ok(data) => {
                data[0]
            }
            Err(e) => return Err(PZEMError::Misc(e.to_string())),
        };
        Ok(data)
    }
    pub async fn get_u32(&mut self, slave_id: u8, addr: Address) -> Result<u32, PZEMError> {
        match self.clone().retry_read_input_registers(addr, 2).await {
            Ok(data) => {
                let val = (data[1] as u32) << 16 | data[0] as u32;
                Ok(val)
            }
            Err(e) => {
                return Err(PZEMError::Misc(e.to_string()));
            }
        }
    }
    //endregion
    pub async fn get_data(&mut self, slave_id: u8) -> Result<PZEMData, PZEMError> {
        self.select_slave(slave_id).await;

        match self.clone().retry_read_input_registers(0x0, 9).await {
            // because holding_registers works in 16 bit "words", we need to combine two words into
            // one word here to get a 32 bit number.
            Ok(data) => {
                let volts = data[0] as f32 / 10_f32;
                let amps = ((data[2] as u32) << 16 | data[1] as u32) as f32 / 1000_f32;
                let watts = ((data[4] as u32) << 16 | data[3] as u32) as f32 / 10_f32;
                let watt_hours = ((data[6] as u32) <<16 | data[5] as u32) as f32;
                let frequency = data[7] as f32 / 10_f32;
                let power_factor = data[8] as f32 / 100_f32;
                //let alarm_status = ((data[10] as u32) <<16 | data[9] as u32) as u32;
                Ok(PZEMData{volts, amps, watts,watt_hours, frequency, power_factor})
            }
            Err(e) => {
                return Err(PZEMError::Misc(e.to_string()));
            }
        }
    }
    //endregion
    pub(crate) async fn retry_read_input_registers(
        self,
        addr: Address,
        q: Quantity,
    ) -> Result<Vec<Word>, PZEMError> {
        let retry_strategy = ExponentialBackoff::from_millis(DEFAULT_BACKOFF_BASE_MS)
            .map(jitter) // add jitter to delays
            .take(1); 

        let ctx = self.ctx.clone();
        match RetryIf::spawn(
            retry_strategy,
            || {
                let future = action_read_input_registers(&ctx, addr, q);
                future
            },
            |e: &PZEMError| PZEMError::TransientError == *e,
        )
            .await
        {
            Ok(e) => Ok(e),
            Err(e) => {
                return Err(e);
            }
        }
    }
    //endregion
}

//region read holding register
pub async fn action_read_input_registers(
    actx: &Arc<Mutex<Box<dyn PZEMConn>>>,
    addr: Address,
    q: Quantity,
) -> Result<Vec<Word>, PZEMError> {
    let mut ctx = actx.lock().await;
    match timeout(
        Duration::from_millis(500),
        ctx.read_input_registers(addr, q),
    )
        .await
    {
        Ok(future) => match future {
            Ok(data) => {
                trace!("{:#x?}", data);
                Ok(data)
            }
            Err(e) => match e.raw_os_error() {
                None => match e.to_string().as_str() {
                    ERROR_ILLEGAL_DATA_VALUE => {
                        return Err(PZEMError::Misc(
                            ERROR_ILLEGAL_DATA_VALUE.to_string(),
                        ));
                    }
                    ERROR_GATEWAY_DEVICE_FAILED_TO_RESPOND => {
                        return Err(PZEMError::TransientError);
                    }
                    _ => {
                        if e.to_string().contains(ERROR_INVALID_RESPONSE_HEADER) {
                            return Err(PZEMError::Misc(String::from(
                                "out of order response",
                            )));
                        };
                        debug!("Non-os specific error: {e}");
                        return Err(PZEMError::TransientError);
                    }
                },
                Some(code) => match code {
                    32 => {
                        return Err(PZEMError::Misc(e.to_string()));
                    }
                    _ => {
                        debug!("OS-specific error: {:#?}", e);
                        return Err(PZEMError::TransientError);
                    }
                },
            },
        },
        Err(e) => {
            debug!("Timeout attempting read: {e}");
            return Err(PZEMError::TransientError);
        }
    }
}
//endregion
