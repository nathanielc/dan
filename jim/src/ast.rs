/// The AST node for expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Get(String),
    String(String),
    Duration(String),
    Ident(String),
}

#[derive(Debug, PartialEq)]
pub enum Stmt {
    Block(Vec<Stmt>),
    Set(String, Expr),
    Let(String, Expr),
    When(String, Expr, Box<Stmt>),
    Wait(Expr, Box<Stmt>),
    Expr(Expr),
    Print(Expr),
}
