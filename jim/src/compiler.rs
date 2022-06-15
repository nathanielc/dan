use crate::ast::{Expr, Stmt};
use crate::Compile;
use anyhow::anyhow;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fmt::Display,
    time::Duration,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Path(String),
    Duration(Duration),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Str(s) => f.write_str(s.as_str()),
            Value::Path(s) => f.write_str(s.as_str()),
            Value::Duration(d) => write!(f, "{:?}", d),
        }
    }
}

impl TryFrom<Value> for String {
    type Error = anyhow::Error;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        match value {
            Value::Str(s) => Ok(s),
            Value::Path(s) => Ok(s),
            _ => Err(anyhow!("value is not a string")),
        }
    }
}

impl TryFrom<Expr> for Value {
    type Error = anyhow::Error;

    fn try_from(value: Expr) -> std::result::Result<Self, Self::Error> {
        match value {
            Expr::String(s) => Ok(Self::Str(s)),
            Expr::Duration(d) => {
                let s = d.strip_suffix("s").unwrap();
                let duration = Duration::from_secs(s.parse().unwrap());
                Ok(Value::Duration(duration))
            }
            _ => Err(anyhow!("expression is not a literal value")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Constant(u16),
    Print,
    Pick(usize),
    Pop,
    Spawn(usize),
    Term,
    When,
    Wait,
    Set,
    Get,
    GetResult,
}

#[derive(Debug, PartialEq)]
pub struct Code {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
}

impl Code {
    fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
        }
    }
}

struct Env<'a> {
    parent: Option<&'a Env<'a>>,
    values: HashMap<String, usize>,
    depth: usize,
}

impl<'a> Env<'a> {
    fn new() -> Env<'a> {
        Env {
            parent: None,
            values: HashMap::new(),
            depth: 0,
        }
    }
    fn nest(&'a self) -> Env<'a> {
        Env {
            parent: Some(self),
            values: HashMap::new(),
            depth: 0,
        }
    }
    fn get_depth(&self, id: &String) -> usize {
        if let Some(depth) = self.values.get(id) {
            self.depth - (*depth)
        } else if let Some(parent) = self.parent {
            self.depth + parent.get_depth(id)
        } else {
            0
        }
    }
}

pub struct Interpreter {
    code: Code,
}

impl Compile for Interpreter {
    type Output = Code;

    fn from_ast(ast: Stmt) -> Self::Output {
        let mut interpreter = Interpreter { code: Code::new() };
        interpreter.interpret_stmt(&mut Env::new(), ast);
        interpreter.add_instruction(Instruction::Term);
        interpreter.code
    }
}

impl Interpreter {
    fn add_constant(&mut self, value: Value) -> u16 {
        self.code.constants.push(value);
        (self.code.constants.len() - 1) as u16 // cast to u16 because that is the size of our constant pool index
    }

    fn add_instruction(&mut self, inst: Instruction) -> u16 {
        let position_of_new_instruction = self.code.instructions.len() as u16;
        self.code.instructions.push(inst);
        position_of_new_instruction
    }
    fn interpret_stmt<'a>(&mut self, env: &mut Env<'a>, stmt: Stmt) {
        match stmt {
            Stmt::Print(expr) => {
                self.interpret_expr(env, expr);
                self.add_instruction(Instruction::Print);
            }
            Stmt::Let(id, expr) => {
                // Compute the value and place it on the stack
                self.interpret_expr(env, expr);
                env.values.insert(id, env.depth);
                env.depth += 1
            }
            Stmt::Block(stmts) => {
                let mut block_env = env.nest();
                for s in stmts {
                    self.interpret_stmt(&mut block_env, s);
                }
                for _ in 0..block_env.depth {
                    self.add_instruction(Instruction::Pop);
                }
            }
            Stmt::When(path, expr, stmt) => {
                let spawn_ip = self.add_instruction(Instruction::Spawn(usize::MAX));
                // Add path
                let const_index = self.add_constant(Value::Path(path));
                self.add_instruction(Instruction::Constant(const_index));
                // Add expr
                self.interpret_expr(env, expr);
                // Watch, creates a promise
                self.add_instruction(Instruction::When);
                // Add stmt
                self.interpret_stmt(env, *stmt);
                // Terminate the spawned thread
                self.add_instruction(Instruction::Term);

                // backpatch the spawn jump pointer
                let l = self.code.instructions.len();
                if let Some(Instruction::Spawn(ip)) =
                    self.code.instructions.get_mut(spawn_ip as usize)
                {
                    *ip = l;
                } else {
                    panic!("missing spawn instruction")
                }
            }
            Stmt::Set(path, expr) => {
                let spawn_ip = self.add_instruction(Instruction::Spawn(usize::MAX));
                // Add path
                let const_index = self.add_constant(Value::Path(path));
                self.add_instruction(Instruction::Constant(const_index));
                // Add expr
                self.interpret_expr(env, expr);
                // Watch, creates a promise
                self.add_instruction(Instruction::Set);
                // Terminate the spawned thread
                self.add_instruction(Instruction::Term);

                // backpatch the spawn jump pointer
                let l = self.code.instructions.len();
                if let Some(Instruction::Spawn(ip)) =
                    self.code.instructions.get_mut(spawn_ip as usize)
                {
                    *ip = l;
                } else {
                    panic!("missing spawn instruction")
                }
            }
            Stmt::Wait(expr, stmt) => {
                let spawn_ip = self.add_instruction(Instruction::Spawn(usize::MAX));
                // Add expr
                self.interpret_expr(env, expr);
                // Wait, creates a promise
                self.add_instruction(Instruction::Wait);
                // Add stmt
                self.interpret_stmt(env, *stmt);
                // Terminate the spawned thread
                self.add_instruction(Instruction::Term);

                // backpatch the spawn jump pointer
                let l = self.code.instructions.len();
                if let Some(Instruction::Spawn(ip)) =
                    self.code.instructions.get_mut(spawn_ip as usize)
                {
                    *ip = l;
                } else {
                    panic!("missing spawn instruction")
                }
            }
            Stmt::Expr(expr) => {
                self.interpret_expr(env, expr);
                self.add_instruction(Instruction::Pop);
            }
        };
    }
    fn interpret_expr<'a>(&mut self, env: &mut Env<'a>, expr: Expr) {
        match expr {
            Expr::Ident(id) => {
                let depth = env.get_depth(&id);
                if depth == 0 {
                    panic!("undefined id");
                }
                self.add_instruction(Instruction::Pick(depth - 1));
            }
            Expr::Get(path) => {
                // Add path
                let const_index = self.add_constant(Value::Path(path));
                self.add_instruction(Instruction::Constant(const_index));
                // Watch, creates a promise
                self.add_instruction(Instruction::Get);
                self.add_instruction(Instruction::GetResult);
            }
            Expr::String(_) | Expr::Duration(_) => {
                let const_index = self.add_constant(expr.try_into().unwrap());
                self.add_instruction(Instruction::Constant(const_index));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        let source = "print \"hello_world\"";
        let code = Interpreter::from_source(source);
        log::debug!("bytecode:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0),
                    Instruction::Print,
                    Instruction::Term,
                ],
                constants: vec![Value::Str("hello_world".to_string())],
            },
            code
        );
    }
    #[test]
    fn test_let() {
        let source = "
let x = \"x\"
let y = \"y\"
let z = \"z\"
print z
print y
print x
print y
print z
";
        let code = Interpreter::from_source(source);
        log::debug!("bytecode:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0), // x
                    Instruction::Constant(1), // y, x
                    Instruction::Constant(2), // z, y, x
                    Instruction::Pick(0),     // z, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pick(1),     // y, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pick(2),     // x, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pick(1),     // y, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pick(0),     // z, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pop,         // y, x
                    Instruction::Pop,         // x
                    Instruction::Pop,         //
                    Instruction::Term,
                ],
                constants: vec![
                    Value::Str("x".to_string()),
                    Value::Str("y".to_string()),
                    Value::Str("z".to_string())
                ],
            },
            code
        );
    }
    #[test]
    fn test_let_block() {
        let source = "
let x = \"x\"
{
    let y = \"y\"
    let z = \"z\"
    print z
    print y
    print x
    print y
    print z
}
";
        let code = Interpreter::from_source(source);
        log::debug!("bytecode:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0), // x
                    Instruction::Constant(1), // y, x
                    Instruction::Constant(2), // z, y, x
                    Instruction::Pick(0),     // z, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pick(1),     // y, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pick(2),     // x, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pick(1),     // y, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pick(0),     // z, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pop,         // y, x
                    Instruction::Pop,         // x
                    Instruction::Pop,         //
                    Instruction::Term,
                ],
                constants: vec![
                    Value::Str("x".to_string()),
                    Value::Str("y".to_string()),
                    Value::Str("z".to_string())
                ],
            },
            code
        );
    }
    #[test]
    fn test_let_blocks() {
        let source = "
let x = \"x\"
{
    let y = \"y\"
    {
        let z = \"z\"
        { print z }
    }
    print y
}
print x
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0), // x
                    Instruction::Constant(1), // y, x
                    Instruction::Constant(2), // z, y, x
                    Instruction::Pick(0),     // z, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pop,         // y, x
                    Instruction::Pick(0),     // y, y, x
                    Instruction::Print,       // y, x
                    Instruction::Pop,         // x
                    Instruction::Pick(0),     // x, x
                    Instruction::Print,       // x
                    Instruction::Pop,         //
                    Instruction::Term,
                ],
                constants: vec![
                    Value::Str("x".to_string()),
                    Value::Str("y".to_string()),
                    Value::Str("z".to_string())
                ],
            },
            code
        );
    }
    #[test]
    fn test_let_shadow() {
        let source = "
let x = \"x\"
{
    let x = \"y\"
    {
        let x = \"z\"
        { print x }
    }
    print x
}
print x
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0), // x
                    Instruction::Constant(1), // y, x
                    Instruction::Constant(2), // z, y, x
                    Instruction::Pick(0),     // z, z, y, x
                    Instruction::Print,       // z, y, x
                    Instruction::Pop,         // y, x
                    Instruction::Pick(0),     // y, y, x
                    Instruction::Print,       // y, x
                    Instruction::Pop,         // x
                    Instruction::Pick(0),     // x, x
                    Instruction::Print,       // x
                    Instruction::Pop,         //
                    Instruction::Term,
                ],
                constants: vec![
                    Value::Str("x".to_string()),
                    Value::Str("y".to_string()),
                    Value::Str("z".to_string())
                ],
            },
            code
        );
    }
    #[test]
    fn test_when() {
        let source = "
        when path is \"off\" print \"off\"
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Spawn(7),
                    Instruction::Constant(0),
                    Instruction::Constant(1),
                    Instruction::When,
                    Instruction::Constant(2),
                    Instruction::Print,
                    Instruction::Term,
                    Instruction::Term,
                ],
                constants: vec![
                    Value::Path("path".to_string()),
                    Value::Str("off".to_string()),
                    Value::Str("off".to_string())
                ],
            },
            code
        );
    }
    #[test]
    fn test_wait() {
        let source = "
        wait 1s print \"done\"
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Spawn(6),
                    Instruction::Constant(0),
                    Instruction::Wait,
                    Instruction::Constant(1),
                    Instruction::Print,
                    Instruction::Term,
                    Instruction::Term,
                ],
                constants: vec![
                    Value::Duration(Duration::from_secs(1)),
                    Value::Str("done".to_string()),
                ],
            },
            code
        );
    }
    #[test]
    fn test_set() {
        let source = "
        set path/to/value \"on\"
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Spawn(5),
                    Instruction::Constant(0),
                    Instruction::Constant(1),
                    Instruction::Set,
                    Instruction::Term,
                    Instruction::Term,
                ],
                constants: vec![
                    Value::Path("path/to/value".to_string()),
                    Value::Str("on".to_string()),
                ],
            },
            code
        );
    }
    #[test]
    fn test_get() {
        let source = "
        get path/to/value
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0),
                    Instruction::Get,
                    Instruction::GetResult,
                    Instruction::Pop,
                    Instruction::Term
                ],
                constants: vec![Value::Path("path/to/value".to_string()),],
            },
            code
        );
    }
}
