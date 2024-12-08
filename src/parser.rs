use anyhow::Result;

use crate::ast;

lalrpop_util::lalrpop_mod!(
    #[allow(clippy::all, missing_debug_implementations, dead_code)]
    parser,
    "/parser.rs"
);

pub fn parse(source: &str) -> Result<ast::Stmt> {
    parser::FileParser::new()
        .parse(source)
        // Map the err tokens to an owned value since otherwise the
        // input would have to live as long as the error which has a static lifetime.
        .map_err(|err| err.map_token(|tok| tok.to_string()).into())
}
#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;
    #[test]
    fn test_ident() {
        let expr = parse("print a;").unwrap();
        expect!([r#"
            [print a;]
        "#]).assert_debug_eq(&expr);

        let expr = parse("print _a;").unwrap();
        expect!([r#"
            [print _a;]
        "#]).assert_debug_eq(&expr);

        let expr = parse("print _a; print b0;").unwrap();
        expect!([r#"
            [print _a; print b0;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_string() {
        let expr = parse(r#"print "string";"#).unwrap();
        expect!([r#"
            [print "string";]
        "#]).assert_debug_eq(&expr);

        let expr = parse(r#"print "string with spaces";"#).unwrap();
        expect!([r#"
            [print "string with spaces";]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_bool() {
        let expr = parse(r#"print true;"#).unwrap();
        expect!([r#"
            [print true;]
        "#]).assert_debug_eq(&expr);

        let expr = parse(r#"print false;"#).unwrap();
        expect!([r#"
            [print false;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_int() {
        let expr = parse(r#"print 42;"#).unwrap();
        expect!([r#"
            [print 42;]
        "#]).assert_debug_eq(&expr);

        let expr = parse(r#"print 0;"#).unwrap();
        expect!([r#"
            [print 0;]
        "#]).assert_debug_eq(&expr);
    }

    #[test]
    fn test_float() {
        let expr = parse(r#"print 42.0;"#).unwrap();
        expect!([r#"
            [print 42.0;]
        "#]).assert_debug_eq(&expr);

        let expr = parse(r#"print 0.0;"#).unwrap();
        expect!([r#"
            [print 0.0;]
        "#]).assert_debug_eq(&expr);

        let expr = parse(r#"print 0.1;"#).unwrap();
        expect!([r#"
            [print 0.1;]
        "#]).assert_debug_eq(&expr);
    }

    #[test]
    fn test_object() {
        let expr = parse(r#"print {value: 42.0};"#).unwrap();
        expect!([r#"
            [print {value: 42.0};]
        "#]).assert_debug_eq(&expr);

        let expr = parse(r#"print {answer: 42.0, question: "how many roads?"};"#).unwrap();
        expect!([r#"
            [print {answer: 42.0, question: "how many roads?"};]
        "#]).assert_debug_eq(&expr);
    }

    #[test]
    fn test_duration() {
        let expr = parse(r#"print 5h;"#).unwrap();
        expect!([r#"
            [print 5h;]
        "#]).assert_debug_eq(&expr);

        let expr = parse(r#"print 1h;print  2m;print  3s;"#).unwrap();
        expect!([r#"
            [print 1h; print 2m; print 3s;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_time() {
        let expr = parse(r#"print 10:05PM;"#).unwrap();
        expect!([r#"
            [print 10:05PM;]
        "#]).assert_debug_eq(&expr);

        let expr = parse(r#"print #sunrise; print #sunset; print 12:25AM;"#).unwrap();
        expect!([r#"
            [print #sunrise; print #sunset; print 12:25AM;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_set() {
        let expr = parse(r#"set [path] 0;"#).unwrap();
        expect!([r#"
            [set path 0;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_let() {
        let expr = parse(r#"let x = 0;"#).unwrap();
        expect!([r#"
            [let x = 0;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_when() {
        let expr = parse(r#"when <path> is 0 print 5;"#).unwrap();
        expect!([r#"
            [when (<path> is 0) print 5;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_as() {
        let expr = parse(r#"print x as y in y;"#).unwrap();
        expect!([r#"
            [print x as y in y;]
        "#]).assert_debug_eq(&expr);
        let expr = parse(r#"print x as a in y as b in b + c;"#).unwrap();
        expect!([r#"
            [print x as a in y as b in (b + c);]
        "#]).assert_debug_eq(&expr);
        let expr = parse(r#"print 1 + 2 * 3 as a in a / 4;"#).unwrap();
        expect!([r#"
            [print (1 + (2 * 3)) as a in (a / 4);]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_wait() {
        let expr = parse(r#"wait 1s print 0;"#).unwrap();
        expect!([r#"
            [wait 1s print 0;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_at() {
        let expr = parse(r#"at x print 0;"#).unwrap();
        expect!([r#"
            [at x print 0;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_print() {
        let expr = parse(r#"print 0;"#).unwrap();
        expect!([r#"
            [print 0;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_scene() {
        let expr = parse(r#"scene a { print 0;};"#).unwrap();
        expect!([r#"
            [scene a [print 0;];]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_start() {
        let expr = parse(r#"start a;"#).unwrap();
        expect!([r#"
            [start a;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_stop() {
        let expr = parse(r#"stop a;"#).unwrap();
        expect!([r#"
            [stop a;]
        "#]).assert_debug_eq(&expr);
    }
    #[test]
    fn test_binary_expr() {
        let expr = parse("").unwrap();
        expect!([r#"
            []
        "#]).assert_debug_eq(&expr);

        let expr = parse("print 22 * 44 + 66;").unwrap();
        expect!([r#"
            [print ((22 * 44) + 66);]
        "#]).assert_debug_eq(&expr);

        let expr = parse("print 22 * 44 + 66;").unwrap();
        expect!([r#"
            [print ((22 * 44) + 66);]
        "#]).assert_debug_eq(&expr);

        let expr = parse("print 22 * 44 + 66;print 13*3;").unwrap();
        expect!([r#"
            [print ((22 * 44) + 66); print (13 * 3);]
        "#]).assert_debug_eq(&expr);

        let expr = parse("print 22 * 44 + 66; print 13*3;").unwrap();
        expect!([r#"
            [print ((22 * 44) + 66); print (13 * 3);]
        "#]).assert_debug_eq(&expr);
    }

    #[test]
    fn test_fail() {
        assert!(parse("@").is_err());
    }
}
