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
    var x: Int
    var y: Int

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
    var x: Int
    var y: Int

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
    let value: Int

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
    var x: Int
    var y: Int

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
    var x: Int

    init(cond: Bool) {
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
    var x: Int

    init(cond: Bool) {
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
    var x: Int

    init(cond: Bool) {
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
    var n: Int

    init(x: Int) {
        if x == 1 {
            self.n = 10;
        } else if x == 2 {
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
    var x: Int
    var y: Int

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
    var x: Int
    var y: Int

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
    var x: Int
    var y: Int

    init(quick: Bool) {
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
    var value: Int

    init(cond: Bool) {
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
    var value: Int

    init(cond: Bool) {
        self.value = 0;
        while cond {
            self.value = self.value + 1;
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
    var n: Int

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
    var x: Int;
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
    let value: Int

    init(cond: Bool) {
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
    let value: Int

    init(cond: Bool) {
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
