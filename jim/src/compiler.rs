use crate::ast::{Expr, Stmt};
use crate::Compile;
use anyhow::anyhow;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCode {
    OpConstant(u16), // pointer to constant table
    OpPrint,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
}

impl TryFrom<Expr> for Value {
    type Error = anyhow::Error;

    fn try_from(value: Expr) -> std::result::Result<Self, Self::Error> {
        match value {
            Expr::String(s) => Ok(Self::Str(s)),
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
        self.code.instructions.push(inst.clone());
        println!(
            "added instructions {:?} from opcode {:?}",
            self.code.instructions, inst,
        );
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
            _ => {}
        };
    }
    fn interpret_expr<'a>(&mut self, env: &mut Env<'a>, expr: Expr) {
        match expr {
            Expr::String(_) => {
                let const_index = self.add_constant(expr.try_into().unwrap());
                self.add_instruction(Instruction::Constant(const_index));
            }
            Expr::Ident(id) => {
                let depth = env.get_depth(&id);
                if depth == 0 {
                    panic!("undefined id");
                }
                self.add_instruction(Instruction::Pick(depth - 1));
            }
            _ => {}
        }
    }
}

fn convert_u16_to_two_u8s(integer: u16) -> [u8; 2] {
    [(integer >> 8) as u8, integer as u8]
}

pub fn convert_two_u8s_to_usize(int1: u8, int2: u8) -> usize {
    ((int1 as usize) << 8) | int2 as usize
}

fn make_three_byte_op(code: u8, data: u16) -> Vec<u8> {
    let mut output = vec![code];
    output.extend(&convert_u16_to_two_u8s(data));
    output
}

pub fn make_op(op: OpCode) -> Vec<u8> {
    match op {
        OpCode::OpConstant(arg) => make_three_byte_op(0x01, arg),
        OpCode::OpPrint => vec![0x02],
    }
}

const STACK_SIZE: usize = 512;

pub struct VM {
    bytecode: Code,
    stack: [Value; STACK_SIZE],
    stack_ptr: usize, // points to the next free space
}

impl VM {
    pub fn new(bytecode: Code) -> Self {
        Self {
            bytecode,
            stack: unsafe { std::mem::zeroed() }, // exercise: This is UB as Node has non-zero discriminant!
            stack_ptr: 0,
        }
    }
    pub fn run(&mut self) {
        let mut ip = 0; // instruction pointer
        while ip < self.bytecode.instructions.len() {
            let inst_addr = ip;
            ip += 1;

            match self.bytecode.instructions[inst_addr] {
                Instruction::Constant(const_idx) => {
                    self.push(self.bytecode.constants[const_idx as usize].clone());
                }
                Instruction::Print => {
                    println!("{:?}", self.pop());
                }
                Instruction::Pick(depth) => {
                    self.pick(depth);
                }
                Instruction::Pop => {
                    self.pop();
                }
                _ => panic!("Unknown instruction"),
            }
        }
    }
    pub fn pick(&mut self, depth: usize) {
        self.push(self.stack[self.stack_ptr - 1 - depth].clone());
    }

    pub fn push(&mut self, value: Value) {
        self.stack[self.stack_ptr] = value;
        self.stack_ptr += 1; // ignoring the potential stack overflow
    }

    pub fn pop(&mut self) -> Value {
        // ignoring the potential of stack underflow
        // cloning rather than mem::replace for easier testing
        let v = self.stack[self.stack_ptr - 1].clone();
        self.stack_ptr -= 1;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        let source = "print \"hello_world\"";
        let code = Interpreter::from_source(source);
        println!("bytecode:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![Instruction::Constant(0), Instruction::Print,],
                constants: vec![Value::Str("hello_world".to_string())],
            },
            code
        );

        let mut vm = VM::new(code);
        vm.run();
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
        println!("bytecode:     {:?}", code);
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
                ],
                constants: vec![
                    Value::Str("x".to_string()),
                    Value::Str("y".to_string()),
                    Value::Str("z".to_string())
                ],
            },
            code
        );

        let mut vm = VM::new(code);
        vm.run();
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
        println!("bytecode:     {:?}", code);
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
                ],
                constants: vec![
                    Value::Str("x".to_string()),
                    Value::Str("y".to_string()),
                    Value::Str("z".to_string())
                ],
            },
            code
        );

        let mut vm = VM::new(code);
        vm.run();
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
        println!("code:     {:?}", code);
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
                ],
                constants: vec![
                    Value::Str("x".to_string()),
                    Value::Str("y".to_string()),
                    Value::Str("z".to_string())
                ],
            },
            code
        );

        let mut vm = VM::new(code);
        vm.run();
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
        println!("code:     {:?}", code);
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
                ],
                constants: vec![
                    Value::Str("x".to_string()),
                    Value::Str("y".to_string()),
                    Value::Str("z".to_string())
                ],
            },
            code
        );

        let mut vm = VM::new(code);
        vm.run();
    }
}
