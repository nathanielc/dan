pub mod ast;
pub mod compiler;
pub mod mqtt_engine;
pub mod parser;
pub mod sun;
pub mod vm;

pub type Result<T> = anyhow::Result<T>;

pub trait Compile {
    type Output;

    fn from_ast(ast: ast::Stmt) -> Self::Output;

    fn from_source(source: &str) -> Result<Self::Output> {
        let ast = parser::parse(source)?;
        Ok(Self::from_ast(ast))
    }
}
