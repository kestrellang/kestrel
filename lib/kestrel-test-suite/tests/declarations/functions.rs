use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn empty_function() {
        Test::new("module Test\nfunc empty() { }")
            .expect(Compiles)
            .expect(
                Symbol::new("empty")
                    .is(SymbolKind::Function)
                    .has(Behavior::ParameterCount(0))
                    .has(Behavior::HasBody(true)),
            );
    }

    #[test]
    fn function_with_return_type() {
        Test::new("module Test\nfunc getValue() -> Int { 42 }")
            .expect(Compiles)
            .expect(
                Symbol::new("getValue")
                    .is(SymbolKind::Function)
                    .has(Behavior::ParameterCount(0))
                    .has(Behavior::HasBody(true)),
            );
    }

    #[test]
    fn function_with_parameters() {
        Test::new("module Test\nfunc add(a: Int, b: Int) -> Int { a + b }")
            .expect(Compiles)
            .expect(
                Symbol::new("add")
                    .is(SymbolKind::Function)
                    .has(Behavior::ParameterCount(2))
                    .has(Behavior::HasBody(true)),
            );
    }

    #[test]
    fn public_function() {
        Test::new("module Test\npublic func publicFn() { }")
            .expect(Compiles)
            .expect(
                Symbol::new("publicFn")
                    .is(SymbolKind::Function)
                    .has(Behavior::Visibility(Visibility::Public))
                    .has(Behavior::ParameterCount(0)),
            );
    }

    #[test]
    fn private_function() {
        Test::new("module Test\nprivate func privateFn() { }")
            .expect(Compiles)
            .expect(
                Symbol::new("privateFn")
                    .is(SymbolKind::Function)
                    .has(Behavior::Visibility(Visibility::Private))
                    .has(Behavior::ParameterCount(0)),
            );
    }

    #[test]
    fn static_function_in_struct() {
        Test::new(
            r#"module Test
            struct Counter {
                static func staticFn() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Counter").is(SymbolKind::Struct))
        .expect(
            Symbol::new("Counter.staticFn")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(true))
                .has(Behavior::ParameterCount(0)),
        );
    }
}

mod overloading {
    use super::*;

    #[test]
    fn overload_by_parameter_count() {
        Test::new(
            r#"module Test
            func process() { }
            func process(x: Int) { }
            func process(x: Int, y: Int) { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("process").is(SymbolKind::Function));
    }

    #[test]
    fn overload_by_parameter_type() {
        Test::new(
            r#"module Test
            func convert(x: Int) -> String { "int" }
            func convert(x: Float) -> String { "float" }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("convert").is(SymbolKind::Function))
        .expect(Symbol::new("convert").has(Behavior::ParameterCount(1)));
    }

    #[test]
    fn overload_by_label() {
        Test::new(
            r#"module Test
            func send(to recipient: String) { }
            func send(from sender: String) { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("send").is(SymbolKind::Function))
        .expect(Symbol::new("send").has(Behavior::ParameterCount(1)));
    }
}

mod in_structs {
    use super::*;

    #[test]
    fn method_in_struct() {
        Test::new(
            r#"module Test
            struct Counter {
                func increment() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Counter").is(SymbolKind::Struct))
        .expect(
            Symbol::new("Counter.increment")
                .is(SymbolKind::Function)
                .has(Behavior::IsInstanceMethod(true))
                .has(Behavior::ParameterCount(0)),
        );
    }

    #[test]
    fn multiple_methods_with_parameters() {
        Test::new(
            r#"module Test
            struct Calculator {
                func add(a: Int, b: Int) -> Int { a + b }
                func subtract(a: Int, b: Int) -> Int { a - b }
                func multiply(a: Int, b: Int) -> Int { a * b }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Calculator").is(SymbolKind::Struct))
        .expect(
            Symbol::new("Calculator.add")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2))
                .has(Behavior::IsInstanceMethod(true)),
        )
        .expect(
            Symbol::new("Calculator.subtract")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        )
        .expect(
            Symbol::new("Calculator.multiply")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        );
    }
}

