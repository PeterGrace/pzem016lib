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

    if let Ok(val) = std::env::var("NEW_SLAVE_ID") {
        let new_slave_id: u8 = val.parse().expect("NEW_SLAVE_ID must be a u8");
        let mut pzem = PZEM::new("10.174.2.48:4196".to_string())
            .await.expect("Can't connect");
        pzem.select_slave(1).await;
        debug!("{:#?}",pzem.set_unit_slave_address(new_slave_id).await);
        let data = pzem.get_data(new_slave_id).await;
        debug!(?data);
    }
    else
    {
        error!("must set NEW_SLAVE_ID");
    }
}
