pub mod ast;
pub mod compiler;
pub mod mqtt_engine;
pub mod vm;

//pub mod sun;

pub type Result<T> = anyhow::Result<T>;

pub trait Compile {
    type Output;

    fn from_ast(ast: ast::Stmt) -> Self::Output;

    fn from_source(source: &str) -> Result<Self::Output> {
        let ast = dan::FileParser::new()
            .parse(source)
            // Map the err tokens to an owned value since otherwise the
            // input would have to live as long as the error which has a static lifetime.
            .map_err(|err| err.map_token(|tok| tok.to_string()))?;
        Ok(Self::from_ast(ast))
    }
}

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(pub dan);

#[cfg(test)]
mod parser {
    use super::*;
    #[test]
    fn test_ident() {
        let expr = dan::FileParser::new().parse("print a;").unwrap();
        assert_eq!(&format!("{:?}", expr), "[print a;]");

        let expr = dan::FileParser::new().parse("print _a;").unwrap();
        assert_eq!(&format!("{:?}", expr), "[print _a;]");

        let expr = dan::FileParser::new().parse("print _a; print b0;").unwrap();
        assert_eq!(&format!("{:?}", expr), "[print _a; print b0;]");
    }
    #[test]
    fn test_string() {
        let expr = dan::FileParser::new().parse(r#"print "string";"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print "string";]"#);

        let expr = dan::FileParser::new()
            .parse(r#"print "string with spaces";"#)
            .unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print "string with spaces";]"#);
    }
    #[test]
    fn test_int() {
        let expr = dan::FileParser::new().parse(r#"print 42;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print 42;]"#);

        let expr = dan::FileParser::new().parse(r#"print 0;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print 0;]"#);
    }

    #[test]
    fn test_float() {
        let expr = dan::FileParser::new().parse(r#"print 42.0;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print 42.0;]"#);

        let expr = dan::FileParser::new().parse(r#"print 0.0;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print 0.0;]"#);

        let expr = dan::FileParser::new().parse(r#"print 0.1;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print 0.1;]"#);
    }

    #[test]
    fn test_object() {
        let expr = dan::FileParser::new()
            .parse(r#"print {value: 42.0};"#)
            .unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print {value: 42.0};]"#);

        let expr = dan::FileParser::new()
            .parse(r#"print {answer: 42.0, question: "how many roads?"};"#)
            .unwrap();
        assert_eq!(
            &format!("{:?}", expr),
            r#"[print {answer: 42.0, question: "how many roads?"};]"#
        );
    }

    #[test]
    fn test_duration() {
        let expr = dan::FileParser::new().parse(r#"print 5h;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print 5h;]"#);

        let expr = dan::FileParser::new()
            .parse(r#"print 1h;print  2m;print  3s;"#)
            .unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print 1h; print 2m; print 3s;]"#);
    }
    #[test]
    fn test_time() {
        let expr = dan::FileParser::new().parse(r#"print 10:05PM;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print 10:05PM;]"#);

        let expr = dan::FileParser::new()
            .parse(r#"print #sunrise; print #sunset; print 12:25AM;"#)
            .unwrap();
        assert_eq!(
            &format!("{:?}", expr),
            r#"[print #sunrise; print #sunset; print 12:25AM;]"#
        );
    }
    #[test]
    fn test_set() {
        let expr = dan::FileParser::new().parse(r#"set [path] 0;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[set path 0;]"#);
    }
    #[test]
    fn test_let() {
        let expr = dan::FileParser::new().parse(r#"let x = 0;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[let x = 0;]"#);
    }
    #[test]
    fn test_when() {
        let expr = dan::FileParser::new()
            .parse(r#"when [path] is 0 print 5;"#)
            .unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[when path is 0 print 5;]"#);
    }
    #[test]
    fn test_wait() {
        let expr = dan::FileParser::new().parse(r#"wait 1s print 0;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[wait 1s print 0;]"#);
    }
    #[test]
    fn test_at() {
        let expr = dan::FileParser::new().parse(r#"at x print 0;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[at x print 0;]"#);
    }
    #[test]
    fn test_print() {
        let expr = dan::FileParser::new().parse(r#"print 0;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[print 0;]"#);
    }
    #[test]
    fn test_scene() {
        let expr = dan::FileParser::new()
            .parse(r#"scene a { print 0;};"#)
            .unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[scene a [print 0;];]"#);
    }
    #[test]
    fn test_start() {
        let expr = dan::FileParser::new().parse(r#"start a;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[start a;]"#);
    }
    #[test]
    fn test_stop() {
        let expr = dan::FileParser::new().parse(r#"stop a;"#).unwrap();
        assert_eq!(&format!("{:?}", expr), r#"[stop a;]"#);
    }
    #[test]
    fn test_binary_expr() {
        let expr = dan::FileParser::new().parse("").unwrap();
        assert_eq!(&format!("{:?}", expr), "[]");

        let expr = dan::FileParser::new().parse("print 22 * 44 + 66;").unwrap();
        assert_eq!(&format!("{:?}", expr), "[print ((22 * 44) + 66);]");

        let expr = dan::FileParser::new().parse("print 22 * 44 + 66;").unwrap();
        assert_eq!(&format!("{:?}", expr), "[print ((22 * 44) + 66);]");

        let expr = dan::FileParser::new()
            .parse("print 22 * 44 + 66;print 13*3;")
            .unwrap();
        assert_eq!(
            &format!("{:?}", expr),
            "[print ((22 * 44) + 66); print (13 * 3);]"
        );

        let expr = dan::FileParser::new()
            .parse("print 22 * 44 + 66; print 13*3;")
            .unwrap();
        assert_eq!(
            &format!("{:?}", expr),
            "[print ((22 * 44) + 66); print (13 * 3);]"
        );
    }

    #[test]
    fn test_fail() {
        assert!(dan::FileParser::new().parse("@").is_err());
    }
}
