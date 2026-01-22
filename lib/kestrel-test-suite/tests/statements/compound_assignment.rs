//! Tests for compound assignment operators.
//!
//! These tests verify that compound assignment operators (+=, -=, *=, /=, %=,
//! &=, |=, ^=, <<=, >>=) are correctly parsed, desugared to protocol methods,
//! and produce the correct type (unit).

use kestrel_test_suite::*;

mod arithmetic_compound_assignment {
    use super::*;

    #[test]
    fn add_assign_basic() {
        // x += 1 should desugar to x.addAssign(1)
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 5;
    x += 1;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn subtract_assign_basic() {
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 10;
    x -= 3;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn multiply_assign_basic() {
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 5;
    x *= 2;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn divide_assign_basic() {
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 10;
    x /= 2;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn modulo_assign_basic() {
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 10;
    x %= 3;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod bitwise_compound_assignment {
    use super::*;

    #[test]
    fn bitwise_and_assign() {
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 7;
    x &= 3;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn bitwise_or_assign() {
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 4;
    x |= 2;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn bitwise_xor_assign() {
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 5;
    x ^= 3;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod shift_compound_assignment {
    use super::*;

    #[test]
    fn left_shift_assign() {
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 1;
    x <<= 3;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn right_shift_assign() {
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 8;
    x >>= 2;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod compound_assignment_returns_unit {
    use super::*;

    #[test]
    fn compound_assignment_has_unit_type() {
        // Compound assignment returns (), so assigning its result to a variable
        // that expects () should work
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 5;
    let result: () = (x += 1);
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn chaining_compound_assignment_fails() {
        // a += b += c should fail because b += c returns () which cannot be
        // used as the operand to +=
        Test::new(
            r#"
module Main

func test() {
    var a: Int = 1;
    var b: Int = 2;
    a += b += 1;
}
"#,
        )
        .with_stdlib()
        .expect(Fails);
    }
}

mod mutability_errors {
    use super::*;

    #[test]
    fn cannot_compound_assign_to_let() {
        // Should fail: x is immutable (let binding)
        Test::new(
            r#"
module Main

func test() {
    let x: Int = 5;
    x += 1;
}
"#,
        )
        .with_stdlib()
        .expect(Fails);
    }

    #[test]
    fn can_compound_assign_to_var() {
        // Should succeed: x is mutable (var binding)
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 5;
    x += 1;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod invalid_targets {
    use super::*;

    #[test]
    fn cannot_compound_assign_to_literal() {
        // Should fail: cannot assign to a literal
        Test::new(
            r#"
module Main

func test() {
    5 += 1;
}
"#,
        )
        .with_stdlib()
        .expect(Fails);
    }
}

mod field_compound_assignment {
    use super::*;

    #[test]
    fn compound_assign_to_var_field() {
        // Should succeed: p is mutable and x is a var field
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

func test() {
    var p = Point(x: 0, y: 0);
    p.x += 10;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn cannot_compound_assign_to_let_field() {
        // Should fail: x is a let field (immutable)
        Test::new(
            r#"
module Main

struct Point {
    let x: Int
    let y: Int
}

func test() {
    var p = Point(x: 0, y: 0);
    p.x += 10;
}
"#,
        )
        .with_stdlib()
        .expect(Fails);
    }

    #[test]
    fn cannot_compound_assign_through_immutable_binding() {
        // Should fail: p is immutable (let binding)
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

func test() {
    let p = Point(x: 0, y: 0);
    p.x += 10;
}
"#,
        )
        .with_stdlib()
        .expect(Fails);
    }
}

mod multiple_compound_assignments {
    use super::*;

    #[test]
    fn sequential_compound_assignments() {
        Test::new(
            r#"
module Main

func test() {
    var x: Int = 0;
    x += 1;
    x += 2;
    x += 3;
    x *= 2;
    x -= 5;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn compound_assignment_with_expressions() {
        // The RHS can be any expression
        Test::new(
            r#"
module Main

func getValue() -> Int { 10 }

func test() {
    var x: Int = 5;
    x += getValue();
    x *= 2 + 3;
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}
