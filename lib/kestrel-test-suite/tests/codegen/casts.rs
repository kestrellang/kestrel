//! Type conversion codegen tests.
//!
//! These tests verify the numeric type conversion methods work correctly.
//! The conversion methods (toInt(), toFloat64(), etc.) provide type-safe
//! conversions between numeric types.
//!
//! NOTE: These tests use primitive type names (I8, I16, I32, F32, F64)
//! rather than the standard library wrapper types (Int8, Int16, etc.)
//! because the test framework doesn't load the standard library.

use super::compile_and_run;

// === Widening Integer Conversions ===
// These tests verify that smaller integer types can be correctly widened to larger ones

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_int_widening_i8_to_i64() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: lang.i8 = 42
    x.toInt()
}
"#,
    );
    if result.exit_code == -1 {
        panic!("Compilation failed: {}", result.stderr);
    }
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_int_widening_i16_to_i64() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: I16 = 100
    x.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 100);
}

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_int_widening_i32_to_i64() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: lang.i32 = 77
    x.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 77);
}

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_int_widening_negative() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: lang.i8 = -10
    let y = x.toInt()
    // -10 as i64 should still be -10
    // exit code wraps to 246 (256 - 10)
    if y == -10 {
        1
    } else {
        0
    }
}
"#,
    );
    assert_eq!(result.exit_code, 1);
}

// === Narrowing Integer Conversions ===
// These tests verify that larger integer types can be truncated to smaller ones

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_int_narrowing_i64_to_i32() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: lang.i64 = 42
    let y = x.toI32()
    y.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_int_narrowing_i64_to_i8() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: lang.i64 = 50
    let y = x.toI8()
    y.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 50);
}

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_int_narrowing_truncation() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    // 256 truncated to i8 should be 0
    let x: lang.i64 = 256
    let y = x.toI8()
    y.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 0);
}

// === lang.f64 to Integer Conversions ===

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_float_to_int() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: F64 = 42.7
    x.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_float_to_int_negative() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: F64 = -5.9
    let y = x.toInt()
    // Should truncate toward zero, so -5
    if y == -5 {
        1
    } else {
        0
    }
}
"#,
    );
    assert_eq!(result.exit_code, 1);
}

// === Integer to Float Conversions ===

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_int_to_float() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: lang.i64 = 42
    let f = x.toF64()
    // Convert back to check
    f.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_int_to_float_arithmetic() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: lang.i64 = 10
    let f = x.toF64()
    let g = f + 0.5
    // 10.5 truncated should be 10
    g.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 10);
}

// === lang.f64 Widening/Narrowing ===

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_float_widening_f32_to_f64() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: F32 = 42.0
    let y = x.toF64()
    y.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_float_narrowing_f64_to_f32() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: F64 = 42.0
    let y = x.toF32()
    let z = y.toF64()
    z.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

// === Chained Conversions ===

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_chained_casts() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: lang.i8 = 42
    let y = x.toI32()
    let z = y.toInt()
    z
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}

#[test]
#[ignore = "conversion methods not yet implemented for primitive types"]
fn test_cast_in_expression() {
    let result = compile_and_run(
        r#"
module Test

func main() -> lang.i64 {
    let x: lang.i8 = 20
    let y: lang.i8 = 22
    x.toInt() + y.toInt()
}
"#,
    );
    assert_eq!(result.exit_code, 42);
}
