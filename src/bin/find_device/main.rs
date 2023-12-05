#[macro_use] extern crate tokio;
#[macro_use] extern crate tracing;

use tokio::time::timeout;
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

    for i in 1..=255 {
        pzem.select_slave(i).await;
        let data = pzem.get_data(i).await;
        info!(?i, ?data);
    }
}
