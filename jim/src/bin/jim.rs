use env_logger;
use jim::{compiler::Interpreter, mqtt_engine::MQTTEngine, vm::VM, Compile, Result};
use std::io::{self, Read};
use tokio::{signal, sync::broadcast};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let (shutdown_tx, shutdown_rx) = broadcast::channel(2);
    tokio::spawn(async move {
        run(input.as_str(), shutdown_rx).await.unwrap();
    });

    match signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            log::error!("unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }
    // Send shutdown signal
    shutdown_tx.send(())?;
    Ok(())
}

async fn run(src: &str, shutdown: broadcast::Receiver<()>) -> Result<()> {
    let mqtt = MQTTEngine::new().unwrap();
    let code = Interpreter::from_source(src);
    let vm = VM::new(mqtt);
    vm.run(code, shutdown).await?;
    Ok(())
}
