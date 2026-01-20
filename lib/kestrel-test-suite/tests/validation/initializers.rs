//! Tests for initializer verification
//!
//! These tests verify that:
//! - All fields must be initialized before the initializer returns
//! - `let` fields can only be assigned once
//! - Fields cannot be read before assigned
//! - Control flow is properly analyzed (if/else, loops, return)

use kestrel_test_suite::*;

mod basic_initialization {
    use super::*;

    #[test]
    fn all_fields_initialized() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init() {
        self.x = 0;
        self.y = 0;
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn missing_field_initialization() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init() {
        self.x = 0;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("does not initialize all fields"));
    }

    #[test]
    fn let_field_double_assignment() {
        Test::new(
            r#"
module Main

struct Id {
    let value: lang.i64

    init() {
        self.value = 1;
        self.value = 2;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("more than once"));
    }

    #[test]
    fn field_read_before_assigned() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init() {
        self.y = self.x;
        self.x = 0;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("before it is initialized"));
    }
}

mod if_else_branches {
    use super::*;

    #[test]
    fn both_branches_initialize() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64

    init(cond: lang.i1) {
        if cond {
            self.x = 1;
        } else {
            self.x = 2;
        }
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn only_then_branch_initializes() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64

    init(cond: lang.i1) {
        if cond {
            self.x = 1;
        }
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("does not initialize all fields"));
    }

    #[test]
    fn then_branch_returns_else_initializes() {
        // If the then branch returns, we only need the else branch to initialize
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64

    init(cond: lang.i1) {
        if cond {
            self.x = 1;
            return;
        }
        self.x = 2;
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn else_if_chain_all_initialize() {
        Test::new(
            r#"
module Main

struct Value {
    var n: lang.i64

    init(x: lang.i64) {
        if lang.i64_eq(x, 1) {
            self.n = 10;
        } else if lang.i64_eq(x, 2) {
            self.n = 20;
        } else {
            self.n = 0;
        }
    }
}
"#,
        )
        .expect(Compiles);
    }
}

mod return_handling {
    use super::*;

    #[test]
    fn return_before_all_initialized() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init() {
        self.x = 0;
        return;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("cannot return before all fields"));
    }

    #[test]
    fn return_after_all_initialized() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init() {
        self.x = 0;
        self.y = 0;
        return;
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn early_return_in_branch_with_later_init() {
        // If the early return branch has all fields initialized, that's fine
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init(quick: lang.i1) {
        if quick {
            self.x = 0;
            self.y = 0;
            return;
        }
        self.x = 1;
        self.y = 2;
    }
}
"#,
        )
        .expect(Compiles);
    }
}

mod while_loops {
    use super::*;

    #[test]
    fn init_only_in_while_body() {
        // While body may not execute, so this should fail
        Test::new(
            r#"
module Main

struct Counter {
    var value: lang.i64

    init(cond: lang.i1) {
        while cond {
            self.value = 0;
        }
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("does not initialize all fields"));
    }

    #[test]
    fn init_before_while() {
        Test::new(
            r#"
module Main

struct Counter {
    var value: lang.i64

    init(cond: lang.i1) {
        self.value = 0;
        while cond {
            self.value = lang.i64_add(self.value, 1);
        }
    }
}
"#,
        )
        .expect(Compiles);
    }
}

mod loop_with_break {
    use super::*;

    #[test]
    fn init_before_break() {
        Test::new(
            r#"
module Main

struct Value {
    var n: lang.i64

    init() {
        loop {
            self.n = 42;
            break;
        }
    }
}
"#,
        )
        .expect(Compiles);
    }
}

mod uninitialized_variables {
    use super::*;

    #[test]
    fn uninitialized_variable_access() {
        // TODO: Add test for uninitialized variables once that validation is implemented
        Test::new(
            r#"
module Main
func test() {
    var x: lang.i64;
    let y = x;
}
"#,
        )
        .expect(HasError("access to uninitialized variable 'x'"));
    }
}

mod let_fields_in_branches {
    use super::*;

    #[test]
    fn let_field_assigned_in_both_branches() {
        // This is allowed - assigned exactly once per path
        Test::new(
            r#"
module Main

struct Id {
    let value: lang.i64

    init(cond: lang.i1) {
        if cond {
            self.value = 1;
        } else {
            self.value = 2;
        }
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn let_field_assigned_then_reassigned_after_merge() {
        // This should fail - assigned in both branches, then again after
        Test::new(
            r#"
module Main

struct Id {
    let value: lang.i64

    init(cond: lang.i1) {
        if cond {
            self.value = 1;
        } else {
            self.value = 2;
        }
        self.value = 3;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("more than once"));
    }
}

mod match_expressions {
    use super::*;

    #[test]
    fn match_with_diverging_branch() {
        // Regression test for: Match in init doesn't prove field initialization
        // When one branch of a match diverges (e.g., panics) and the other initializes,
        // the field should be considered initialized
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(T)
    case None
}

struct Container[T] {
    var ptr: lang.ptr[T]

    init(maybeValue: Option[lang.ptr[T]]) {
        match maybeValue {
            .Some(rawPtr) => {
                self.ptr = rawPtr;
            },
            .None => lang.panic("allocation failed")
        }
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_all_arms_initialize() {
        // When all arms of a match initialize the field, it should be considered initialized
        Test::new(
            r#"
module Main

enum Result[T, E] {
    case Ok(T)
    case Err(E)
}

struct Container[T] {
    var value: T

    init(result: Result[T, lang.i64]) {
        match result {
            .Ok(v) => {
                self.value = v;
            },
            .Err(_) => {
                self.value = lang.panic("failed");
            }
        }
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_not_all_arms_initialize() {
        // When not all arms initialize the field, it should fail
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(T)
    case None
}

struct Container[T] {
    var ptr: lang.ptr[T]

    init(maybeValue: Option[lang.ptr[T]]) {
        match maybeValue {
            .Some(rawPtr) => {
                self.ptr = rawPtr;
            },
            .None => {
                // This branch doesn't initialize ptr and doesn't diverge
            }
        }
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("does not initialize all fields"));
    }
}

mod extension_initializers {
    use super::*;

    #[test]
    fn initializer_in_extension_can_be_called() {
        // Initializers defined in extensions should be callable
        Test::new(
            r#"
module Main

public struct Foo {
    var x: lang.i64
    public init() { self.x = 0; }
}

extend Foo {
    public init(value: lang.i64) {
        self.x = value;
    }
}

public func test() {
    let f = Foo(value: 42);
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_initializers_in_extension() {
        // Multiple initializers can be defined in extensions
        Test::new(
            r#"
module Main

public struct Point {
    var x: lang.i64
    var y: lang.i64
}

extend Point {
    public init(x: lang.i64, y: lang.i64) {
        self.x = x;
        self.y = y;
    }

    public init(value: lang.i64) {
        self.x = value;
        self.y = value;
    }
}

public func test() {
    let p1 = Point(x: 1, y: 2);
    let p2 = Point(value: 5);
}
"#,
        )
        .expect(Compiles);
    }
}
