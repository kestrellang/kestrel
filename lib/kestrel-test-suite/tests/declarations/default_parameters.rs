//! Tests for default function parameters.
//!
//! Default parameters allow function, initializer, and subscript parameters
//! to specify default values that are used when the caller omits the argument.

use kestrel_test_suite::*;

mod syntax {
    use super::*;

    #[test]
    fn basic_default_parameter() {
        Test::new(
            r#"
module Main

func greet(name: lang.str = "World") { }
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("greet")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn multiple_default_parameters() {
        Test::new(
            r#"
module Main

func createPoint(x: lang.i64 = 0, y: lang.i64 = 0) { }
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("createPoint")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        );
    }

    #[test]
    fn mixed_required_and_default() {
        Test::new(
            r#"
module Main

func divide(numerator: lang.i64, denominator: lang.i64 = 1) -> lang.i64 {
    denominator
}
"#,
        )
        .expect(Compiles)
        .expect(
            // Use full path to avoid collision with builtin 'divide' method on Int/Float
            Symbol::new("Main.divide")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        );
    }

    #[test]
    fn labeled_parameter_with_default() {
        Test::new(
            r#"
module Main

func createUser(with name: lang.str = "Anonymous") { }
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("createUser")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn default_with_expression() {
        // Default can be any expression, not just literals
        Test::new(
            r#"
module Main

func getDefault() -> lang.i64 { 42 }

func process(value: lang.i64 = getDefault()) { }
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("process")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }
}

mod calls {
    use super::*;

    #[test]
    fn call_with_default_omitted() {
        Test::new(
            r#"
module Main

func greet(name: lang.str = "World") -> lang.str {
    name
}

func test() -> lang.str {
    greet()
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn call_with_default_provided() {
        Test::new(
            r#"
module Main

func greet(name: lang.str = "World") -> lang.str {
    name
}

func test() -> lang.str {
    greet("Alice")
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn call_with_multiple_defaults_all_omitted() {
        Test::new(
            r#"
module Main

func createPoint(x: lang.i64 = 0, y: lang.i64 = 0) -> lang.i64 {
    lang.i64_add(x, y)
}

func test() -> lang.i64 {
    createPoint()
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn call_with_multiple_defaults_partial() {
        Test::new(
            r#"
module Main

func createPoint(x: lang.i64 = 0, y: lang.i64 = 0) -> lang.i64 {
    lang.i64_add(x, y)
}

func test() -> lang.i64 {
    createPoint(10)
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn call_with_required_and_default() {
        Test::new(
            r#"
module Main

func compute(numerator: lang.i64, denominator: lang.i64 = 1) -> lang.i64 {
    lang.i64_signed_div(numerator, denominator)
}

func test() -> lang.i64 {
    compute(42)
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn call_with_labeled_default_omitted() {
        Test::new(
            r#"
module Main

func send(to recipient: lang.str = "default@example.com") { }

func test() {
    send()
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn call_with_labeled_default_provided() {
        Test::new(
            r#"
module Main

func send(to recipient: lang.str = "default@example.com") { }

func test() {
    send(to: "alice@example.com")
}
"#,
        )
        .expect(Compiles);
    }
}

mod errors {
    use super::*;

    #[test]
    fn required_after_default_is_error() {
        Test::new(
            r#"
module Main

func invalid(a: lang.i64 = 0, b: lang.i64) { }
"#,
        )
        .expect(HasError("required parameter"));
    }

    #[test]
    fn duplicate_signature_with_defaults() {
        // Signatures ignore defaults, so these are duplicates
        Test::new(
            r#"
module Main

func foo(x: lang.i64) { }
func foo(x: lang.i64 = 0) { }
"#,
        )
        .expect(HasError("duplicate"));
    }

    #[test]
    fn default_type_mismatch() {
        Test::new(
            r#"
module Main

func bad(x: lang.i64 = "not an int") { }
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }
}

mod initializers {
    use super::*;

    #[test]
    fn init_with_default_parameter() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init(x: lang.i64 = 0, y: lang.i64 = 0) {
        self.x = x;
        self.y = y;
    }
}

func test() -> Point {
    Point()
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_partial_defaults() {
        Test::new(
            r#"
module Main

struct Rectangle {
    var width: lang.i64
    var height: lang.i64

    init(width: lang.i64, height: lang.i64 = 100) {
        self.width = width;
        self.height = height;
    }
}

func test() -> Rectangle {
    Rectangle(50)
}
"#,
        )
        .expect(Compiles);
    }
}

mod subscripts {
    use super::*;

    #[test]
    fn subscript_with_default_parameter() {
        Test::new(
            r#"
module Main

struct Container {
    var value: lang.i64

    init(value: lang.i64) {
        self.value = value;
    }

    subscript(index: lang.i64 = 0) -> lang.i64 {
        get { value }
    }
}

func test() -> lang.i64 {
    let c = Container(42);
    c()
}
"#,
        )
        .expect(Compiles);
    }
}

mod generics {
    use super::*;

    #[test]
    fn generic_function_with_default() {
        Test::new(
            r#"
module Main

func first[T](items: T, fallback: T = items) -> T {
    fallback
}
"#,
        )
        // This should error because defaults can't reference other params
        .expect(HasError("cannot reference"));
    }
}
