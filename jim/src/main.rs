use std::fs;

mod frontend;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let foo: String = fs::read_to_string("test.jim")?;
    let f = frontend::parser::file(&foo)?;
    println!("AST: {:?}", f);
    Ok(())
}
