// test: diagnostics
// stdlib: false

module Test
            protocol Parser {
                type Output = lang.str;
                func parse() -> Output
            }
            struct SimpleParser: Parser {
                func parse() -> lang.str { "" }
            }
