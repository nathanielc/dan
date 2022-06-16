use env_logger;
use jim::{
    compiler::Interpreter,
    mqtt_engine::MQTTEngine,
    vm::{Engine, VM},
    Compile, Result,
};
use std::io::{self, Read};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let mqtt = MQTTEngine::new()?;

    run(input.as_str(), mqtt).await?;

    Ok(())
}

async fn run<E: Engine + 'static>(src: &str, engine: E) -> Result<()> {
    let code = Interpreter::from_source(src);
    let vm = VM::new(engine);
    vm.run(code).await?;
    Ok(())
}
