use lexpr::{self, Cons, Value};
use std::io;

pub enum WASMExpr {
    Module(Vec<WASMExpr>),
    Func(Vec<WASMExpr>),
    Param(Type, Option<String>),
    Result(Type, Option<String>),
    Local(Type, Option<String>),
    Cmd(String),
    Number(i64),
    Name(String),
    Call,
}

pub enum Type {
    I32,
    I64,
    F32,
    F64,
}

pub fn to_sexpr(wexpr: &WASMExpr) -> Value {
    match wexpr {
        WASMExpr::Module(exprs) => Cons::new(symbol("module"), vec_to_sexpr(exprs)).into(),
        WASMExpr::Func(exprs) => Cons::new(symbol("func"), vec_to_sexpr(exprs)).into(),
        WASMExpr::Param(typ, name) => def_to_sexpr("param", typ, name),
        WASMExpr::Result(typ, name) => def_to_sexpr("result", typ, name),
        WASMExpr::Local(typ, name) => def_to_sexpr("local", typ, name),
        WASMExpr::Cmd(name) => Value::Symbol(name.clone().into_boxed_str()),
        WASMExpr::Number(n) => Value::Number((*n).into()),
        WASMExpr::Name(name) => Value::Symbol(name.clone().into_boxed_str()),
        WASMExpr::Call => symbol("call"),
    }
}

impl From<&Type> for Value {
    fn from(t: &Type) -> Self {
        match t {
            Type::I32 => symbol("i32"),
            Type::I64 => symbol("i64"),
            Type::F32 => symbol("f32"),
            Type::F64 => symbol("f64"),
        }
    }
}

fn vec_to_sexpr(vw: &Vec<WASMExpr>) -> Value {
    iter_to_sexpr(vw.iter())
}

fn iter_to_sexpr<'a, I>(iter: I) -> Value
where
    I: DoubleEndedIterator<Item = &'a WASMExpr>,
{
    iter.rev().fold(Value::Null, |acc, expr| {
        Cons::new(to_sexpr(expr), acc).into()
    })
}

fn def_to_sexpr(sym: &'static str, typ: &Type, name: &Option<String>) -> Value {
    if let Some(n) = name {
        Cons::new(
            symbol(sym),
            Cons::new(
                Value::Symbol(n.clone().into_boxed_str()),
                Cons::new(typ, Value::Null),
            ),
        )
        .into()
    } else {
        Cons::new(symbol(sym), Cons::new(typ, Value::Null)).into()
    }
}

// Symbol converts a static string into a s-expression symbol.
fn symbol(str: &'static str) -> Value {
    Value::Symbol(String::from(str).into_boxed_str())
}

pub fn to_string(wexpr: &WASMExpr) -> io::Result<String> {
    let vec = to_vec(wexpr)?;
    let string = unsafe {
        // We do not emit invalid UTF-8.
        String::from_utf8_unchecked(vec)
    };
    Ok(string)
}
pub fn to_vec(wexpr: &WASMExpr) -> io::Result<Vec<u8>> {
    let mut writer = Vec::with_capacity(128);
    to_writer(&mut writer, wexpr)?;
    Ok(writer)
}
pub fn to_writer<W: io::Write>(writer: W, wexpr: &WASMExpr) -> io::Result<()> {
    let sexpr = to_sexpr(wexpr);
    lexpr::to_writer(writer, &sexpr)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_empty_module() {
        let e = WASMExpr::Module(vec![]);
        assert_eq!("(module)", to_string(&e).unwrap());
    }
    #[test]
    fn test_module() {
        let e = WASMExpr::Module(vec![WASMExpr::Func(vec![]), WASMExpr::Func(vec![])]);
        assert_eq!("(module (func) (func))", to_string(&e).unwrap());
    }
    #[test]
    fn test_empty_func() {
        let e = WASMExpr::Func(vec![]);
        assert_eq!("(func)", to_string(&e).unwrap());
    }
    #[test]
    fn test_func() {
        let e = WASMExpr::Func(vec![
            WASMExpr::Param(Type::I32, None),
            WASMExpr::Param(Type::F32, None),
            WASMExpr::Result(Type::I64, None),
            WASMExpr::Local(Type::I64, None),
            WASMExpr::Cmd("local.get".to_string()),
            WASMExpr::Number(0),
            WASMExpr::Cmd("local.get".to_string()),
            WASMExpr::Number(1),
            WASMExpr::Cmd("local.get".to_string()),
            WASMExpr::Number(2),
        ]);
        assert_eq!(
            "(func (param i32) (param f32) (result i64) (local i64) local.get 0 local.get 1 local.get 2)",
            to_string(&e).unwrap()
        );
    }
    #[test]
    fn test_func_named() {
        let e = WASMExpr::Func(vec![
            WASMExpr::Param(Type::I32, Some("$a".to_string())),
            WASMExpr::Param(Type::F32, Some("$b".to_string())),
            WASMExpr::Result(Type::I64, None),
            WASMExpr::Local(Type::I64, Some("$x".to_string())),
            WASMExpr::Cmd("local.get".to_string()),
            WASMExpr::Name("$a".to_string()),
            WASMExpr::Cmd("local.get".to_string()),
            WASMExpr::Name("$b".to_string()),
            WASMExpr::Cmd("local.get".to_string()),
            WASMExpr::Name("$x".to_string()),
            WASMExpr::Call,
            WASMExpr::Name("$y".to_string()),
        ]);
        assert_eq!(
            "(func (param $a i32) (param $b f32) (result i64) (local $x i64) local.get $a local.get $b local.get $x call $y)",
            to_string(&e).unwrap()
        );
    }
}
