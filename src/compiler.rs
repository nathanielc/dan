use crate::ast::{BinaryOpcode, Expr, Stmt};
use crate::Compile;
use anyhow::anyhow;
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    convert::{TryFrom, TryInto},
    fmt::Display,
    time::Duration,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Value {
    Str(String),
    Path(String),
    Duration(Duration),
    Time(TimeOfDay),
    Float(f64),
    Integer(i64),
    Bool(bool),
    Object(BTreeMap<String, Value>),
    Jump(usize),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Str(s) => f.write_str(s.as_str()),
            Value::Path(s) => f.write_str(s.as_str()),
            Value::Duration(d) => write!(f, "{:?}", d),
            Value::Time(t) => write!(f, "{}", t),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Jump(ip) => write!(f, "jmp: {:?}", ip),
            Value::Object(props) => {
                write!(f, "{{")?;
                for (i, (k, v)) in props.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
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
impl TryFrom<Value> for Vec<u8> {
    type Error = anyhow::Error;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        match value {
            Value::Str(s) => Ok(s.as_bytes().to_vec()),
            Value::Path(s) => Ok(s.as_bytes().to_vec()),
            Value::Duration(_) => todo!(),
            Value::Time(_) => todo!(),
            Value::Float(f) => Ok(f.to_string().as_bytes().to_vec()),
            Value::Integer(i) => Ok(i.to_string().as_bytes().to_vec()),
            Value::Bool(_) => todo!(),
            Value::Jump(_) => todo!(),
            Value::Object(props) => {
                let json = serde_json::to_vec(&props)?;
                Ok(json)
            }
        }
    }
}
impl TryFrom<&[u8]> for Value {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let v: serde_json::Value = serde_json::from_slice(value)?;
        if let Some(v) = json_to_value(v) {
            Ok(v)
        } else {
            Ok(Value::Str(String::from_utf8(value.to_vec())?))
        }
    }
}
fn json_to_value(v: serde_json::Value) -> Option<Value> {
    match v {
        serde_json::Value::Bool(b) => Some(Value::Bool(b)),
        serde_json::Value::Number(f) => {
            if f.is_i64() {
                Some(Value::Integer(f.as_i64().unwrap()))
            } else if f.is_f64() {
                Some(Value::Float(f.as_f64().unwrap()))
            } else {
                None
            }
        }
        serde_json::Value::String(s) => Some(Value::Str(s)),
        serde_json::Value::Null => None,
        serde_json::Value::Array(_) => None,
        serde_json::Value::Object(jprops) => {
            let mut props = BTreeMap::<String, Value>::new();
            for (k, jv) in jprops {
                if let Some(v) = json_to_value(jv) {
                    props.insert(k, v);
                } else {
                    return None;
                }
            }
            Some(Value::Object(props))
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
            Expr::Time(t) => match t.as_str() {
                "sunrise" => Ok(Value::Time(TimeOfDay::Sunrise)),
                "sunset" => Ok(Value::Time(TimeOfDay::Sunset)),
                _ => {
                    let mut hours = 0;
                    let time = if let Some(time) = t.strip_suffix("PM") {
                        hours += 12;
                        time
                    } else if let Some(time) = t.strip_suffix("AM") {
                        time
                    } else {
                        panic!("parser failed to enforce AM/PM ending to time")
                    };
                    let parts: Vec<&str> = time.split(":").collect();
                    if parts.len() != 2 {
                        panic!("parser failed to HH:MM time format")
                    }
                    let h: u32 = parts
                        .first()
                        .unwrap()
                        .parse()
                        .expect("parser failed to enforce integer hours");
                    if h == hours {
                        // 12PM is noon
                        hours = 0;
                    }
                    let m: u32 = parts
                        .last()
                        .unwrap()
                        .parse()
                        .expect("parser failed to enforce integer minutes");

                    Ok(Value::Time(TimeOfDay::HM(hours + h, m)))
                }
            },
            Expr::Float(n) => Ok(Value::Float(n)),
            Expr::Boolean(n) => Ok(Value::Bool(n)),
            Expr::Integer(n) => Ok(Value::Integer(n)),
            Expr::Object(props) => {
                let mut properties = BTreeMap::new();
                for (key, expr) in props {
                    properties.insert(key, expr.try_into()?);
                }
                Ok(Value::Object(properties))
            }
            _ => Err(anyhow!("expression is not a literal value")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum TimeOfDay {
    Sunrise,
    Sunset,
    HM(u32, u32),
}

impl Display for TimeOfDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeOfDay::Sunrise => f.write_str("sunrise"),
            TimeOfDay::Sunset => f.write_str("sunset"),
            TimeOfDay::HM(h, m) => write!(f, "{}:{}", h, m),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Constant(usize),
    Print,
    Pick(usize),
    Pop,
    Swap,
    Spawn(usize),
    Jump(usize),
    JmpNot(usize),
    Call,
    Return,
    Term,
    Wait,
    At,
    Set,
    Stop,
    SceneContext,
    Get,
    Equal,
    Index,
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
    fn add_constant(&mut self, value: Value) -> usize {
        self.code.constants.push(value);
        self.code.constants.len() - 1
    }

    fn add_instruction(&mut self, inst: Instruction) -> usize {
        let position_of_new_instruction = self.code.instructions.len();
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
            Stmt::When(expr, stmt) => {
                let spawn_ip = self.add_instruction(Instruction::Spawn(usize::MAX));
                // Add expr
                self.interpret_expr(env, expr);
                // Add Conditional Jump
                self.add_instruction(Instruction::JmpNot(spawn_ip as usize + 1));
                // Add stmt
                self.interpret_stmt(env, *stmt);
                // Loop the spawned thread back to the beginning
                self.add_instruction(Instruction::Jump(spawn_ip as usize + 1));

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
            Stmt::Set(path, expr) => {
                let const_index = self.add_constant(Value::Path(path));
                self.add_instruction(Instruction::Constant(const_index));
                // Add expr
                self.interpret_expr(env, expr);
                // Watch, creates a promise
                self.add_instruction(Instruction::Set);
            }
            Stmt::Expr(expr) => {
                self.interpret_expr(env, expr);
                self.add_instruction(Instruction::Pop);
            }
            Stmt::Scene(id, stmt) => {
                // Scenes are an implicit definition of two functions:
                // a start and a stop function.
                env.values.insert(id.clone(), env.depth);
                env.depth += 1;
                let start_jump_const =
                    self.add_constant(Value::Jump(self.code.instructions.len() + 3));
                self.add_instruction(Instruction::Constant(start_jump_const));

                env.values.insert(id + " stop", env.depth);
                env.depth += 1;
                let stop_jump_const = self.add_constant(Value::Jump(usize::MAX)); // we need to backpatch this jump location
                self.add_instruction(Instruction::Constant(stop_jump_const));

                let continue_jump = self.add_instruction(Instruction::Jump(usize::MAX)); // we need to backpatch this jump location

                // Add scene body
                self.add_instruction(Instruction::SceneContext);
                self.interpret_stmt(env, *stmt);
                self.add_instruction(Instruction::Return);

                // Add scene stop body
                let stop_jump_ip = self.add_instruction(Instruction::Stop);
                self.add_instruction(Instruction::Return);

                // Backpatch jump constant
                if let Some(Value::Jump(ip)) = self.code.constants.get_mut(stop_jump_const as usize)
                {
                    *ip = stop_jump_ip as usize;
                } else {
                    panic!("missing stop jump value")
                }

                // Backpatch the continue jump pointer
                let l = self.code.instructions.len();
                if let Some(Instruction::Jump(ip)) = self.code.instructions.get_mut(continue_jump) {
                    *ip = l;
                } else {
                    panic!("missing continue jump instruction")
                }
            }
            Stmt::Start(id) => {
                self.interpret_expr(env, Expr::Ident(id));
                self.add_instruction(Instruction::Call);
            }
            Stmt::Stop(id) => {
                self.interpret_expr(env, Expr::Ident(id + " stop"));
                self.add_instruction(Instruction::Call);
            }
            Stmt::At(expr, stmt) => {
                let spawn_ip = self.add_instruction(Instruction::Spawn(usize::MAX));
                self.interpret_expr(env, expr);
                self.add_instruction(Instruction::At);
                self.interpret_stmt(env, *stmt);

                // Loop the spawned thread back to the beginning
                self.add_instruction(Instruction::Jump(spawn_ip as usize + 1));

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
            Expr::Binary(lhs, op, rhs) => {
                self.interpret_expr(env, *lhs);
                self.interpret_expr(env, *rhs);
                match op {
                    BinaryOpcode::Eql => self.add_instruction(Instruction::Equal),
                    _ => todo!(),
                };
            }
            Expr::Path(p) => {
                let path = self.add_constant(Value::Path(p));
                self.add_instruction(Instruction::Constant(path));
                self.add_instruction(Instruction::Get);
            }
            Expr::String(_)
            | Expr::Duration(_)
            | Expr::Time(_)
            | Expr::Float(_)
            | Expr::Boolean(_)
            | Expr::Integer(_)
            | Expr::Object(_) => {
                let const_index = self.add_constant(expr.try_into().unwrap());
                self.add_instruction(Instruction::Constant(const_index));
            }
            Expr::As(init, id, cont) => {
                // Compute the value and place it on the stack
                self.interpret_expr(env, *init);

                // Create new scope block for this value
                let mut block_env = env.nest();
                block_env.values.insert(id, block_env.depth);
                block_env.depth += 1;
                self.interpret_expr(&mut block_env, *cont);
                self.add_instruction(Instruction::Swap);
                self.add_instruction(Instruction::Pop);
            }
            Expr::Index(expr, prop) => {
                // Compute the value and place it on the stack
                self.interpret_expr(env, *expr);
                let p = self.add_constant(Value::Str(prop));
                self.add_instruction(Instruction::Constant(p));
                self.add_instruction(Instruction::Index);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use macro_map::btree_map;

    use super::*;

    #[test]
    fn test_hello_world() {
        let source = r#"print "hello_world";"#;
        let code = Interpreter::from_source(source).unwrap();
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
        let source = r#"
let x = "x";
let y = "y";
let z = "z";
print z;
print y;
print x;
print y;
print z;
"#;
        let code = Interpreter::from_source(source).unwrap();
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
        let source = r#"
let x = "x";
{
    let y = "y";
    let z = "z";
    print z;
    print y;
    print x;
    print y;
    print z;
};
"#;
        let code = Interpreter::from_source(source).unwrap();
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
        let source = r#"
let x = "x";
{
    let y = "y";
    {
        let z = "z";
        { print z; };
    };
    print y;
};
print x;
"#;
        let code = Interpreter::from_source(source).unwrap();
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
        let source = r#"
let x = "x";
{
    let x = "y";
    {
        let x = "z";
        { print x; };
    };
    print x;
};
print x;
"#;
        let code = Interpreter::from_source(source).unwrap();
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
    fn test_as() {
        let source = r#"
        print 1 as x in x;
"#;
        let code = Interpreter::from_source(source).unwrap();
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0),
                    Instruction::Pick(0),
                    Instruction::Swap,
                    Instruction::Pop,
                    Instruction::Print,
                    Instruction::Term,
                ],
                constants: vec![Value::Integer(1)],
            },
            code
        );
    }
    #[test]
    fn test_index() {
        let source = r#"
        let o = {x: 1};
        print o.x;
"#;
        let code = Interpreter::from_source(source).unwrap();
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0),
                    Instruction::Pick(0),
                    Instruction::Constant(1),
                    Instruction::Index,
                    Instruction::Print,
                    Instruction::Pop,
                    Instruction::Term,
                ],
                constants: vec![
                    Value::Object(btree_map![
                        "x".to_string() => Value::Integer(1)
                    ]),
                    Value::Str("x".to_string()),
                ],
            },
            code
        );
    }
    #[test]
    fn test_when() {
        let source = r#"
        when <path> is "off" print "off";
"#;
        let code = Interpreter::from_source(source).unwrap();
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Spawn(9),
                    Instruction::Constant(0),
                    Instruction::Get,
                    Instruction::Constant(1),
                    Instruction::Equal,
                    Instruction::JmpNot(1),
                    Instruction::Constant(2),
                    Instruction::Print,
                    Instruction::Jump(1),
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
    fn test_when_as() {
        let source = r#"
        when <path> as x in x is "off" print "off";
"#;
        let code = Interpreter::from_source(source).unwrap();
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Spawn(12),
                    Instruction::Constant(0),
                    Instruction::Get,
                    Instruction::Pick(0),
                    Instruction::Constant(1),
                    Instruction::Equal,
                    Instruction::Swap,
                    Instruction::Pop,
                    Instruction::JmpNot(1),
                    Instruction::Constant(2),
                    Instruction::Print,
                    Instruction::Jump(1),
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
        let source = r#"
        wait 1s print "done";
"#;
        let code = Interpreter::from_source(source).unwrap();
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
        let source = r#"
        set [path/to/value] "on";
"#;
        let code = Interpreter::from_source(source).unwrap();
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0),
                    Instruction::Constant(1),
                    Instruction::Set,
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
    fn test_scene() {
        let source = r#"
        scene night { print "x"; };
        start night;
        stop night;
"#;
        let code = Interpreter::from_source(source).unwrap();
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0), // Jump address of scene start code
                    Instruction::Constant(1), // Jump address of scene stop code
                    Instruction::Jump(9),
                    Instruction::SceneContext, // Scene start
                    Instruction::Constant(2),
                    Instruction::Print,
                    Instruction::Return,
                    Instruction::Stop, // Scene stop
                    Instruction::Return,
                    Instruction::Pick(1), // Start
                    Instruction::Call,
                    Instruction::Pick(0), // Stop
                    Instruction::Call,
                    Instruction::Pop, // pop the scene start out of scope
                    Instruction::Pop, // pop the scene stop out of scope
                    Instruction::Term
                ],
                constants: vec![Value::Jump(3), Value::Jump(7), Value::Str("x".to_string()),],
            },
            code
        );
    }
    #[test]
    fn test_at() {
        let source = r#"
        at 12:50PM print "x";
"#;
        let code = Interpreter::from_source(source).unwrap();
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Spawn(6),
                    Instruction::Constant(0),
                    Instruction::At,
                    Instruction::Constant(1),
                    Instruction::Print,
                    Instruction::Jump(1),
                    Instruction::Term,
                ],
                constants: vec![
                    Value::Time(TimeOfDay::HM(12, 50)),
                    Value::Str("x".to_string()),
                ],
            },
            code
        );
    }
    #[test]
    fn test_bool() {
        let source = r#"
        print true;
"#;
        let code = Interpreter::from_source(source).unwrap();
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0),
                    Instruction::Print,
                    Instruction::Term,
                ],
                constants: vec![Value::Bool(true),],
            },
            code
        );
    }
    #[test]
    fn test_float() {
        let source = r#"
        print 7.0;
"#;
        let code = Interpreter::from_source(source).unwrap();
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0),
                    Instruction::Print,
                    Instruction::Term,
                ],
                constants: vec![Value::Float(7.0),],
            },
            code
        );
    }
    #[test]
    fn test_integer() {
        let source = r#"
        print 7;
"#;
        let code = Interpreter::from_source(source).unwrap();
        log::debug!("code:     {:?}", code);
        assert_eq!(
            Code {
                instructions: vec![
                    Instruction::Constant(0),
                    Instruction::Print,
                    Instruction::Term,
                ],
                constants: vec![Value::Integer(7),],
            },
            code
        );
    }
}
