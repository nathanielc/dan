use env_logger;
use jim::{compiler::Interpreter, mqtt_engine::MQTTEngine, vm::VM, Compile, Result};
use std::{
    io::{self, Read},
    sync::Arc,
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let code = Interpreter::from_source(input.as_str());

    let mut mqtt = MQTTEngine::new()?;
    mqtt.connect().await?;

    let mqtt = Arc::new(mqtt);

    {
        let mut vm = VM::new(code, mqtt.clone());
        vm.run();
    }

    Arc::try_unwrap(mqtt).unwrap().disconnect().await?;
    Ok(())
}
