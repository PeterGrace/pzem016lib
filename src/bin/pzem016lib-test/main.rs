#[macro_use] extern crate tokio;
#[macro_use] extern crate tracing;

use tokio_modbus::Slave;
use tracing_subscriber::filter::EnvFilter;

use pzem016lib::{PZEM, action_read_input_registers};
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let mut pzem = PZEM::new("10.174.2.48:4196".to_string())
        .await.expect("Can't connect");

    //let mut ctx = pzem.ctx.lock().await;
    //ctx.set_slave(Slave(240_u8));
    //ctx.write_single_register(0x2,2).await;

    for i in 101..104 {
        let data = pzem.get_data(i).await;
         debug!(?i,?data);
    }
}
