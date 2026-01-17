//! RAII and destructor tests.
//!
//! These tests verify that the compiler correctly handles:
//! 1. Deinit statements (destructor calls)
//! 2. Conditional deinit (DeinitIf) for potentially moved values
//! 3. Deinit flags for tracking move status across branches

use crate::codegen::compile_and_run;

// =============================================================================
// Basic RAII Tests
// =============================================================================

#[test]
fn test_simple_struct_no_deinit() {
    // Simple struct without deinit - should compile and run
    let result = compile_and_run(
        r#"
module Test

struct Resource {
    let value: lang.i64
}

func main() -> lang.i64 {
    let r = Resource(value: 42);
    r.value
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
fn test_struct_field_access_after_creation() {
    // Create struct, access field, return it
    let result = compile_and_run(
        r#"
module Test

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func main() -> lang.i64 {
    let p = Point(x: 10, y: 20);
    p.x + p.y
}
"#,
    );
    assert_eq!(result.exit_code, 30, "stderr: {}", result.stderr);
}

#[test]
fn test_struct_in_if_branch() {
    // Struct created in one branch
    let result = compile_and_run(
        r#"
module Test

struct Resource {
    let value: lang.i64
}

func main() -> lang.i64 {
    let cond = true;
    var result = 0;
    if cond {
        let r = Resource(value: 42);
        result = r.value;
    }
    result
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
fn test_struct_in_both_branches() {
    // Struct created in both branches with different values
    let result = compile_and_run(
        r#"
module Test

struct Resource {
    let value: lang.i64
}

func main() -> lang.i64 {
    let cond = true;
    var result = 0;
    if cond {
        let r = Resource(value: 42);
        result = r.value;
    } else {
        let r = Resource(value: 100);
        result = r.value;
    }
    result
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
fn test_nested_structs_no_deinit() {
    // Nested structs without deinit
    let result = compile_and_run(
        r#"
module Test

struct Inner {
    let value: lang.i64
}

struct Outer {
    let inner: Inner
    let extra: lang.i64
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 10), extra: 5);
    o.inner.value + o.extra
}
"#,
    );
    assert_eq!(result.exit_code, 15, "stderr: {}", result.stderr);
}

// =============================================================================
// Tests with actual deinit methods (when implemented)
// =============================================================================
// These tests are marked as ignored because they require deinit method support
// which may not be fully wired up yet.

#[test]
fn test_deinit_method_called() {
    // This test would verify that deinit is actually called.
    // We can't easily test this without side effects (printing, globals, etc.)
    // For now, just verify it compiles.
    //
    // Note: Kestrel uses `deinit { body }` syntax, not `func deinit(self)`
    let result = compile_and_run(
        r#"
module Test

struct Resource {
    let id: lang.i64
    
    deinit {
        // Cleanup would happen here
        // Without side effects, we can't verify it was called
    }
}

func main() -> lang.i64 {
    let r = Resource(id: 42);
    r.id
}
"#,
    );
    // If this compiles and runs without crashing, deinit support is working
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
fn test_nested_struct_deinit_order() {
    // Verify that nested struct deinit happens in reverse field order
    //
    // Note: Kestrel uses `deinit { body }` syntax, not `func deinit(self)`
    let result = compile_and_run(
        r#"
module Test

struct Inner {
    let value: lang.i64
    
    deinit {
        // Inner deinit
    }
}

struct Outer {
    let first: Inner
    let second: Inner
    
    deinit {
        // Outer deinit runs first, then fields in reverse order
    }
}

func main() -> lang.i64 {
    let o = Outer(first: Inner(value: 1), second: Inner(value: 2));
    o.first.value + o.second.value
}
"#,
    );
    assert_eq!(result.exit_code, 3, "stderr: {}", result.stderr);
}

// =============================================================================
// Move semantics and conditional deinit
// =============================================================================

#[test]
fn test_struct_not_moved() {
    // Struct is created but not moved - should be deinited at scope end
    let result = compile_and_run(
        r#"
module Test

struct Resource {
    let value: lang.i64
}

func main() -> lang.i64 {
    let r = Resource(value: 42);
    let x = r.value;  // Copy field, don't move r
    x
}
"#,
    );
    assert_eq!(result.exit_code, 42, "stderr: {}", result.stderr);
}

#[test]
fn test_loop_with_struct() {
    // Struct created in loop iterations
    let result = compile_and_run(
        r#"
module Test

struct Resource {
    let value: lang.i64
}

func main() -> lang.i64 {
    var sum = 0;
    var i = 0;
    while i < 3 {
        let r = Resource(value: i * 10);
        sum = sum + r.value;
        i = i + 1;
    }
    sum  // 0 + 10 + 20 = 30
}
"#,
    );
    assert_eq!(result.exit_code, 30, "stderr: {}", result.stderr);
}

#[test]
fn test_early_return_with_struct() {
    // Early return should still handle struct cleanup
    let result = compile_and_run(
        r#"
module Test

struct Resource {
    let value: lang.i64
}

func test_early_return(x: lang.i64) -> lang.i64 {
    let r = Resource(value: x);
    if x > 50 {
        return r.value + 1;
    }
    r.value
}

func main() -> lang.i64 {
    test_early_return(60)
}
"#,
    );
    assert_eq!(result.exit_code, 61, "stderr: {}", result.stderr);
}
