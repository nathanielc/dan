use env_logger;
use jim::{compiler::Interpreter, mqtt_engine::MQTTEngine, vm::VM, Compile, Result};
use std::io::{self, Read};
use tokio::{select, signal, sync::broadcast};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let (shutdown_tx, shutdown_rx) = broadcast::channel(2);
    let mut shutdown_rx2 = shutdown_rx.resubscribe();
    let shutdown_tx2 = shutdown_tx.clone();
    tokio::spawn(async move {
        run(input.as_str(), shutdown_rx).await.unwrap();
        shutdown_tx2.send(()).unwrap();
    });

    select! {
        sig = signal::ctrl_c() => {
            match sig {
                Ok(()) => {}
                Err(err) => {
                    log::error!("unable to listen for shutdown signal: {}", err);
                    // we also shut down in case of error
                }
            }
        }
        _ = shutdown_rx2.recv() => {}
    };
    // Send shutdown signal
    let _ = shutdown_tx.send(());
    Ok(())
}

async fn run(src: &str, shutdown: broadcast::Receiver<()>) -> Result<()> {
    let mqtt = MQTTEngine::new().unwrap();
    let code = Interpreter::from_source(src);
    let vm = VM::new(mqtt);
    vm.run(code, shutdown).await?;
    Ok(())
}
