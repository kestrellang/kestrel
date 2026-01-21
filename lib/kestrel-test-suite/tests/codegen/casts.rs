//! Type conversion codegen tests.
//!
//! These tests verify the lang.cast_* intrinsics work correctly for
//! converting between numeric types.

use kestrel_test_suite::*;

// === Widening Integer Conversions ===
// These tests verify that smaller integer types can be correctly widened to larger ones

#[test]
fn test_int_widening_i8_to_i64() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.i8 = 42;
    let result = lang.cast_i8_i64(x);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_int_widening_i16_to_i64() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.i16 = 100;
    let result = lang.cast_i16_i64(x);
    if lang.i64_eq(result, 100) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_int_widening_i32_to_i64() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.i32 = 77;
    let result = lang.cast_i32_i64(x);
    if lang.i64_eq(result, 77) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_int_widening_negative() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.i8 = lang.i8_neg(10);
    let y = lang.cast_i8_i64(x);
    // -10 as i64 should still be -10
    let neg10: lang.i64 = lang.i64_neg(10);
    if lang.i64_eq(y, neg10) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// === Narrowing Integer Conversions ===
// These tests verify that larger integer types can be truncated to smaller ones

#[test]
fn test_int_narrowing_i64_to_i32() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.i64 = 42;
    let y = lang.cast_i64_i32(x);
    let result = lang.cast_i32_i64(y);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_int_narrowing_i64_to_i8() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.i64 = 50;
    let y = lang.cast_i64_i8(x);
    let result = lang.cast_i8_i64(y);
    if lang.i64_eq(result, 50) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_int_narrowing_truncation() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    // 256 truncated to i8 should be 0
    let x: lang.i64 = 256;
    let y = lang.cast_i64_i8(x);
    let result = lang.cast_i8_i64(y);
    if lang.i64_eq(result, 0) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// === Float to Integer Conversions ===

#[test]
fn test_float_to_int() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.f64 = 42.7;
    let result = lang.cast_f64_i64(x);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_float_to_int_negative() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.f64 = lang.f64_neg(5.9);
    let y = lang.cast_f64_i64(x);
    // Should truncate toward zero, so -5
    let neg5: lang.i64 = lang.i64_neg(5);
    if lang.i64_eq(y, neg5) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// === Integer to Float Conversions ===

#[test]
fn test_int_to_float() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.i64 = 42;
    let f = lang.cast_i64_f64(x);
    // Convert back to check
    let result = lang.cast_f64_i64(f);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_int_to_float_arithmetic() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.i64 = 10;
    let f = lang.cast_i64_f64(x);
    let g = lang.f64_add(f, 0.5);
    // 10.5 truncated should be 10
    let result = lang.cast_f64_i64(g);
    if lang.i64_eq(result, 10) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// === Float Widening/Narrowing ===

#[test]
fn test_float_widening_f32_to_f64() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.f32 = 42.0;
    let y = lang.cast_f32_f64(x);
    let result = lang.cast_f64_i64(y);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_float_narrowing_f64_to_f32() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.f64 = 42.0;
    let y = lang.cast_f64_f32(x);
    let z = lang.cast_f32_f64(y);
    let result = lang.cast_f64_i64(z);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// === Chained Conversions ===

#[test]
fn test_chained_casts() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.i8 = 42;
    let y = lang.cast_i8_i32(x);
    let result = lang.cast_i32_i64(y);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_cast_in_expression() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let x: lang.i8 = 20;
    let y: lang.i8 = 22;
    let result = lang.i64_add(lang.cast_i8_i64(x), lang.cast_i8_i64(y));
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
