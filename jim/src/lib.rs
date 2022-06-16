pub mod ast;
pub mod compiler;
pub mod mqtt_engine;
pub mod vm;

pub type Result<T> = anyhow::Result<T>;

pub trait Compile {
    type Output;

    fn from_ast(ast: ast::Stmt) -> Self::Output;

    fn from_source(source: &str) -> Self::Output {
        let ast: ast::Stmt = parser::file(source).unwrap();
        Self::from_ast(ast)
    }
}

use crate::ast::*;
peg::parser!(pub grammar parser() for str {
    pub rule file() -> Stmt
        = _ b:statement()* _ { Stmt::Block(b) }

    rule statement() -> Stmt
        = block()
        / set()
        / print()
        / let()
        / when()
        / wait()
        / e:expression() { Stmt::Expr(e) }

    rule block() -> Stmt
        = _ "{" _ b:(statement()*) _ "}" _ { Stmt::Block(b) }

    rule set() -> Stmt
        =  _ "set" _ pm:path_match() _ e:expression() _ { Stmt::Set(pm, e) }

    rule print() -> Stmt
        =  _ "print" _ e:expression() _ { Stmt::Print(e) }

    rule let() -> Stmt
        =  _ "let" _ i:identifier() _ "=" _ e:expression() _  { Stmt::Let(i, e) }

    rule when() -> Stmt
        =  _ "when" _
            pm:path_match()
            _ "is" _
            e:expression()
            s:statement() _ { Stmt::When(pm, e,  Box::new(s)) }

    rule wait() -> Stmt
        =  _ "wait" _
            d:duration() _
            s:statement() _ { Stmt::Wait(d, Box::new(s)) }

    rule expression() -> Expr
        = get()
        / string()
        / duration()
        / i:identifier() {Expr::Ident(i)}

    rule get() -> Expr
        =  _ "get" _ p:path() _  { Expr::Get(p) }

    rule string() -> Expr
        = "\"" v:$(['0'..='9'| 'a'..='z' | 'A'..='Z' | '_' ]+) "\"" { Expr::String(v.to_owned()) }

    rule duration() -> Expr
        = d:$(['0'..='9']+ ("h"/"m"/"s")) { Expr::Duration(d.to_owned()) }


    rule path_match() -> String
        = pm:$(
            "$" /
            ((("+" / identifier()) "/")* ("+" / "#" / identifier()))
        ) { pm.to_owned() }

    rule path() -> String
        = p:$(
            "$" /
            (( identifier() "/")* (identifier()))
        ) { p.to_owned() }


    rule identifier() -> String
        = quiet!{ n:$(['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '0'..='9' | '_']*) { n.to_owned() } }
        / expected!("identifier")

    rule _() =  quiet!{[' ' | '\t' | '\n']*}
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::file;

    #[test]
    fn test_hello_world() {
        let source = "print \"hello_world\"";
        let ast: Stmt = file(source).unwrap();
        log::debug!("{:?}", ast);
    }
}