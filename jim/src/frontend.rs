pub mod fronted;

/// The AST node for expressions.
#[derive(Debug, PartialEq)]
pub enum Expr {
    Block(Vec<Expr>),
    Set(String, String),
    Get(String),
    Assign(String, Box<Expr>),
    When(String, String, String, Box<Expr>),
}

peg::parser!(pub grammar parser() for str {
    pub rule block() -> Expr
        = _ "{" _ "\n"? _ b:(statement()*) _ "}" _ "\n"? { Expr::Block(b) }
        / _ s:statement() _ { Expr::Block(vec![s]) }
    rule statement() -> Expr
        = _ e:expression() _ "\n"? { e }

    rule expression() -> Expr
        = set()
        / get()
        / assign()
        / when()

    rule set() -> Expr
        =  _ "set" _ pm:path_match() _ v:value() _ { Expr::Set(pm, v) }

    rule get() -> Expr
        =  _ "get" _ p:path() _  { Expr::Get(p) }

    rule assign() -> Expr
        =  _ "var" _ i:identifier() _ "=" _ e:expression() _  { Expr::Assign(i, Box::new(e)) }

    rule when() -> Expr
        =  _ "when" _
            pm:path_match()
            _ "is" _
            v:value()
            _ "wait" _
            d:duration()
            _ b:block() _ { Expr::When(pm, v, d, Box::new(b)) }

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

    rule duration() -> String
        = d:$(['0'..='9']+ ("h"/"m"/"s")) { d.to_owned() }

    rule identifier() -> String
        = quiet!{ n:$(['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '0'..='9' | '_']*) { n.to_owned() } }
        / expected!("identifier")

    rule value() -> String
        = v:$(['0'..='9'| 'a'..='z' | 'A'..='Z' ]+) { v.to_owned() }

    rule _() =  quiet!{[' ' | '\t']*}
});

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
