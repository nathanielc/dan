use jim::{
    compiler::{Interpreter, VM},
    Compile, Result,
};
use std::io;

fn main() -> Result<()> {
    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                println!("{n} bytes read");
                println!("{input}");
            }
            Err(error) => println!("error: {error}"),
        }
        let code = Interpreter::from_source(input.as_str());
        println!("bytecode:     {:?}", code);

        let mut vm = VM::new(code);
        vm.run();
    }
}
