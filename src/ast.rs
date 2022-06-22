/// The AST node for expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(String),
    Get(String),
    String(String),
    Duration(String),
    Time(String),
}

#[derive(Debug, PartialEq)]
pub enum Stmt {
    Block(Vec<Stmt>),
    Set(String, Expr),
    Let(String, Expr),
    When(String, Expr, Box<Stmt>),
    Wait(Expr, Box<Stmt>),
    At(Expr, Box<Stmt>),
    Expr(Expr),
    Print(Expr),
    Scene(String, Box<Stmt>),
    Start(String),
    Stop(String),
    Func(String, Vec<String>, Box<Stmt>),
}
