use env_logger;
use jim::{compiler::Interpreter, mqtt_engine::MQTTEngine, vm::VM, Compile, Result};
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::{select, signal, sync::broadcast};

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// URL to MQTT broker
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long, default_value = "mqtt://localhost")]
    mqtt_url: String,

    /// Input file
    #[structopt(short, long, parse(from_os_str))]
    file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt = Opt::from_args();
    log::debug!("options {:?}", opt);

    let source = read_input(opt.file)?;
    let url = opt.mqtt_url.clone();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (completion_tx, mut completion_rx) = broadcast::channel(1);
    tokio::spawn(async move {
        run(source.as_str(), url.as_str(), shutdown_rx)
            .await
            .unwrap();
        completion_tx.send(()).unwrap();
    });

    // Wait for user supplied signal or for the program to run to completion.
    select! {
        sig = signal::ctrl_c() => {
            match sig {
                Ok(()) => {}
                Err(err) => {
                    log::error!("unable to listen for shutdown signal: {}", err);
                    // we also shut down in case of error
                }
            }
            // Send shutdown signal
            shutdown_tx.send(())?;
        }
        _ = completion_rx.recv() => {}
    };
    Ok(())
}

async fn run(src: &str, url: &str, shutdown: broadcast::Receiver<()>) -> Result<()> {
    let mqtt = MQTTEngine::new(url).unwrap();
    let code = Interpreter::from_source(src);
    let vm = VM::new(mqtt);
    vm.run(code, shutdown).await?;
    Ok(())
}

fn read_input(f: Option<PathBuf>) -> Result<String> {
    match f {
        Some(path) => Ok(fs::read_to_string(path)?),
        None => {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            Ok(input)
        }
    }
}
