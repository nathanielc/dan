use anyhow::anyhow;
use env_logger;
use jim::{compiler::Interpreter, mqtt_engine::MQTTEngine, vm::VM, Compile, Result};
use std::path::PathBuf;
use std::{fs, sync::Arc};
use structopt::StructOpt;
use tokio::{select, signal, sync::broadcast, task::JoinSet};

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// URL to MQTT broker
    #[structopt(short, long, default_value = "mqtt://localhost", env = "JIM_MQTT_URL")]
    mqtt_url: String,

    /// Input directory
    #[structopt(
        short,
        long,
        parse(from_os_str),
        default_value = "jim.d",
        env = "JIM_DIR"
    )]
    dir: PathBuf,
}

const JIM_EXT: &str = "jim";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt = Opt::from_args();
    log::debug!("options {:?}", opt);

    let mqtt = MQTTEngine::new(&opt.mqtt_url)?;
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let mut join_set = JoinSet::new();

    for entry in fs::read_dir(opt.dir)? {
        let entry = entry?;
        if entry.path().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ext == JIM_EXT {
                    let source = fs::read_to_string(entry.path())?;
                    let mqtt = mqtt.clone();
                    let shutdown_rx = shutdown_rx.resubscribe();
                    let path = entry.path().clone();
                    join_set.spawn(async move {
                        log::debug!("running file: {}", path.display());
                        let code = Interpreter::from_source(&source)?;
                        let vm = VM::new(mqtt);
                        vm.run(code, shutdown_rx).await?;
                        log::debug!("finished file: {} ", path.display());
                        Ok(()) as Result<()>
                    });
                }
            }
        }
    }

    // Wait for user supplied signal or for the program to run to completion.
    loop {
        select! {
            // Wait for shutdown signal
            sig = signal::ctrl_c() => {
                sig?;
                // Send shutdown to all tasks
                shutdown_tx.send(())?;
                break;
            }
            // Wait for task and error it any task encounters an error
            res = join_set.join_next() => {
                if let Some(res) = res {
                    res??;
                } else {
                    // All tasks have finished
                    break;
                }
            }
        };
    }
    // Drain all tasks, they should shutdown gracefully at this point
    while let Some(res) = join_set.join_next().await {
        res??;
    }

    // Cleanup mqtt
    if let Ok(mqtt) = Arc::try_unwrap(mqtt) {
        mqtt.shutdown().await?;
        Ok(())
    } else {
        Err(anyhow!("not all threads stopped"))
    }
}
