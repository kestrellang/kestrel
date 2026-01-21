//! Tests for dead code detection
//!
//! These tests verify that unreachable code is detected:
//! - Code after return
//! - Code after break/continue
//! - Code after infinite loop

use kestrel_test_suite::*;

mod after_return {
    use super::*;

    #[test]
    fn code_after_return_warns() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    return 42;
    let x: lang.i64 = 1;
    x
}
"#,
        )
        .expect(Compiles)
        .expect(HasWarning("unreachable"));
    }

    #[test]
    fn code_after_return_with_value_warns() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    return lang.i64_add(1, 2);
    return 3;
}
"#,
        )
        .expect(Compiles)
        .expect(HasWarning("unreachable"));
    }

    #[test]
    fn no_code_after_return_no_warning() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    return 42
}
"#,
        )
        .expect(Compiles)
        .expect(NoWarnings);
    }

    #[test]
    fn return_in_if_branch_code_after_is_reachable() {
        // Code after if with return in only one branch is reachable
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        return 1;
    }
    42
}
"#,
        )
        .expect(Compiles)
        .expect(NoWarnings);
    }

    #[test]
    fn return_in_both_branches_code_after_unreachable() {
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        return 1;
    } else {
        return 2;
    }
    42
}
"#,
        )
        .expect(Compiles)
        .expect(HasWarning("unreachable"));
    }
}

mod after_break_continue {
    use super::*;

    #[test]
    fn code_after_break_in_loop_warns() {
        Test::new(
            r#"
module Main

func test() {
    loop {
        break;
        let x: lang.i64 = 1;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(HasWarning("unreachable"));
    }

    #[test]
    fn code_after_continue_in_loop_warns() {
        Test::new(
            r#"
module Main

func test() {
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        i = lang.i64_add(i, 1);
        continue;
        let x: lang.i64 = 1;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(HasWarning("unreachable"));
    }

    #[test]
    fn break_at_end_of_loop_no_warning() {
        Test::new(
            r#"
module Main

func test() {
    loop {
        let x: lang.i64 = 1;
        break;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(NoWarnings);
    }
}

mod infinite_loop {
    use super::*;

    #[test]
    fn code_after_infinite_loop_warns() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    loop {
        ()
    }
    42
}
"#,
        )
        .expect(Compiles)
        .expect(HasWarning("unreachable"));
    }

    #[test]
    fn loop_with_break_code_after_reachable() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    loop {
        break;
    }
    42
}
"#,
        )
        .expect(Compiles)
        .expect(NoWarnings);
    }

    #[test]
    fn loop_with_conditional_break_code_after_reachable() {
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    loop {
        if cond {
            break;
        }
    }
    42
}
"#,
        )
        .expect(Compiles)
        .expect(NoWarnings);
    }

    #[test]
    fn loop_with_return_code_after_unreachable() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    loop {
        return 1;
    }
    42
}
"#,
        )
        .expect(Compiles)
        .expect(HasWarning("unreachable"));
    }
}

mod nested_control_flow {
    use super::*;

    #[test]
    fn nested_return_in_loop() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        if lang.i64_eq(i, 5) {
            return i;
        }
        i = lang.i64_add(i, 1);
    }
    0
}
"#,
        )
        .expect(Compiles)
        .expect(NoWarnings);
    }

    #[test]
    fn dead_code_in_nested_if() {
        Test::new(
            r#"
module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    if a {
        if b {
            return 1;
        } else {
            return 2;
        }
        let x: lang.i64 = 3;
    }
    0
}
"#,
        )
        .expect(Compiles)
        .expect(HasWarning("unreachable"));
    }
}
