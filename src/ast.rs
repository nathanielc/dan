use std::fmt::{Debug, Error, Formatter};

/// The AST node for expressions.
#[derive(Clone, PartialEq)]
pub enum Expr {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Binary(Box<Expr>, BinaryOpcode, Box<Expr>),
    Ident(String),
    String(String),
    Object(Vec<(String, Expr)>),
    Duration(String),
    Time(String),
    Path(String),
    As(Box<Expr>, String, Box<Expr>),
    Index(Box<Expr>, String),
}
impl Debug for Expr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        match self {
            Expr::Boolean(b) => write!(fmt, "{b:?}"),
            Expr::Integer(i) => write!(fmt, "{i:?}"),
            Expr::Float(f) => write!(fmt, "{f:?}"),
            Expr::Binary(l, op, r) => write!(fmt, "({l:?} {op:?} {r:?})"),
            Expr::Ident(i) => write!(fmt, "{i}"),
            Expr::String(s) => write!(fmt, "{s:?}"),
            Expr::Object(props) => {
                write!(fmt, "{{")?;
                for (i, (k, v)) in props.iter().enumerate() {
                    if i > 0 {
                        write!(fmt, ", ")?;
                    }
                    write!(fmt, "{k}: {v:?}")?;
                }
                write!(fmt, "}}")
            }
            Expr::Duration(d) => write!(fmt, "{d}"),
            Expr::Time(t) => write!(fmt, "{t}"),
            Expr::Path(p) => write!(fmt, "<{p}>"),
            Expr::As(init, name, cont) => write!(fmt, "{init:?} as {name} in {cont:?}",),
            Expr::Index(obj, prop) => write!(fmt, "{obj:?}.{prop}",),
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum BinaryOpcode {
    Mul,
    Div,
    Add,
    Sub,
    Eql,
}

impl Debug for BinaryOpcode {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        match self {
            BinaryOpcode::Mul => write!(fmt, "*"),
            BinaryOpcode::Div => write!(fmt, "/"),
            BinaryOpcode::Add => write!(fmt, "+"),
            BinaryOpcode::Sub => write!(fmt, "-"),
            BinaryOpcode::Eql => write!(fmt, "is"),
        }
    }
}

#[derive(PartialEq)]
pub enum Stmt {
    Block(Vec<Stmt>),
    Set(String, Expr),
    Let(String, Expr),
    When(Expr, Box<Stmt>),
    //Once(String, Expr, Box<Stmt>),
    Wait(Expr, Box<Stmt>),
    At(Expr, Box<Stmt>),
    Expr(Expr),
    Print(Expr),
    Scene(String, Box<Stmt>),
    Start(String),
    Stop(String),
    //Func(String, Vec<String>, Box<Stmt>),
}

impl Debug for Stmt {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        match self {
            Stmt::Block(stmts) => {
                write!(fmt, "[")?;
                for (i, s) in stmts.iter().enumerate() {
                    if i > 0 {
                        write!(fmt, " ")?;
                    }
                    write!(fmt, "{:?};", s)?;
                }
                write!(fmt, "]")
            }
            Stmt::Set(path, expr) => write!(fmt, "set {} {:?}", path, expr),
            Stmt::Expr(expr) => write!(fmt, "{:?}", expr),
            Stmt::Let(id, expr) => write!(fmt, "let {} = {:?}", id, expr),
            Stmt::When(expr, body) => write!(fmt, "when {:?} {:?}", expr, body),
            Stmt::Wait(expr, body) => write!(fmt, "wait {:?} {:?}", expr, body),
            Stmt::At(expr, body) => write!(fmt, "at {:?} {:?}", expr, body),
            Stmt::Print(expr) => write!(fmt, "print {:?}", expr),
            Stmt::Scene(id, body) => write!(fmt, "scene {} {:?}", id, body),
            Stmt::Start(id) => write!(fmt, "start {}", id),
            Stmt::Stop(id) => write!(fmt, "stop {}", id),
        }
    }
}
