/// The AST node for expressions.
#[derive(Debug, PartialEq)]
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
}

#[derive(Debug, PartialEq)]
pub struct File {
    stmts: Vec<Stmt>,
}

peg::parser!(pub grammar parser() for str {
    pub rule file() -> File
        = _ s:statement()* _ { File{stmts: s} }

    rule statement() -> Stmt
        = block()
        / set()
        / let()
        / when()
        / wait()
        / e:expression() { Stmt::Expr(e) }

    rule block() -> Stmt
        = _ "{" _ b:(statement()*) _ "}" _ { Stmt::Block(b) }

    rule set() -> Stmt
        =  _ "set" _ pm:path_match() _ e:expression() _ { Stmt::Set(pm, e) }

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
/*
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_block() {
        let src = "{ set foo bar }";
        let b = parser::block(&src).expect("must parse");
        assert_eq!(
            Expr::Block(vec![Expr::Set("foo".to_string(), "bar".to_string())]),
            b
        )
    }
    #[test]
    fn test_set() {
        let src = "set foo bar";
        let b = parser::block(&src).expect("must parse");
        assert_eq!(
            Expr::Block(vec![Expr::Set("foo".to_string(), "bar".to_string())]),
            b
        )
    }
    #[test]
    fn test_get() {
        let src = "get foo";
        let b = parser::block(&src).expect("must parse");
        assert_eq!(Expr::Block(vec![Expr::Get("foo".to_string())]), b)
    }
    #[test]
    fn test_assign() {
        let src = "var x = get foo";
        let b = parser::block(&src).expect("must parse");
        assert_eq!(
            Expr::Block(vec![Expr::Assign(
                "x".to_string(),
                Box::new(Expr::Get("foo".to_string()))
            )]),
            b
        )
    }
    #[test]
    fn test_when() {
        let src = "when foo is bar wait 1m set $ off";
        let b = parser::block(&src).expect("must parse");
        assert_eq!(
            Expr::Block(vec![Expr::When(
                "foo".to_string(),
                "bar".to_string(),
                "1m".to_string(),
                Box::new(Expr::Block(vec![Expr::Set(
                    "$".to_string(),
                    "off".to_string(),
                )])),
            )]),
            b
        )
    }
    #[test]
    fn test_path_match() {
        let src = "set foo/+/bar/# baz";
        let b = parser::block(&src).expect("must parse");
        assert_eq!(
            Expr::Block(vec![Expr::Set(
                "foo/+/bar/#".to_string(),
                "baz".to_string()
            )]),
            b
        )
    }
}
*/
