//! Tests for the @platform attribute.
//!
//! These tests verify that @platform(.darwin) / @platform(.linux) correctly
//! includes or excludes declarations based on the compilation target.

use kestrel_test_suite::*;

// =============================================================================
// PLATFORM MATCHING (current platform)
// =============================================================================

mod current_platform {
    use super::*;

    #[test]
    fn function_with_matching_platform_compiles() {
        let platform = if cfg!(target_os = "macos") {
            "darwin"
        } else {
            "linux"
        };
        Test::new(&format!(
            r#"module Test
            @platform(.{platform})
            func foo() -> lang.i64 {{ 42 }}
        "#
        ))
        .expect(Compiles);
    }

    #[test]
    fn struct_with_matching_platform_compiles() {
        let platform = if cfg!(target_os = "macos") {
            "darwin"
        } else {
            "linux"
        };
        Test::new(&format!(
            r#"module Test
            @platform(.{platform})
            struct Foo {{
                var x: lang.i64
            }}
        "#
        ))
        .expect(Compiles);
    }

    #[test]
    fn enum_with_matching_platform_compiles() {
        let platform = if cfg!(target_os = "macos") {
            "darwin"
        } else {
            "linux"
        };
        Test::new(&format!(
            r#"module Test
            @platform(.{platform})
            enum Color {{ case Red }}
        "#
        ))
        .expect(Compiles);
    }

    #[test]
    fn function_without_platform_always_compiles() {
        Test::new(
            r#"module Test
            func foo() -> lang.i64 { 42 }
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// PLATFORM EXCLUSION (non-matching platform)
// =============================================================================

mod exclusion {
    use super::*;

    #[test]
    fn non_matching_platform_function_excluded() {
        // Use the opposite platform — this function should be excluded
        let other_platform = if cfg!(target_os = "macos") {
            "linux"
        } else {
            "darwin"
        };
        // If the function were included, calling it would work.
        // Since it's excluded, calling it should fail with an error.
        Test::new(&format!(
            r#"module Test
            @platform(.{other_platform})
            func excluded() -> lang.i64 {{ 42 }}

            func main() {{
                let x = excluded();
            }}
        "#
        ))
        .expect(HasError("excluded"));
    }

    #[test]
    fn non_matching_platform_struct_excluded() {
        let other_platform = if cfg!(target_os = "macos") {
            "linux"
        } else {
            "darwin"
        };
        Test::new(&format!(
            r#"module Test
            @platform(.{other_platform})
            struct ExcludedStruct {{
                var x: lang.i64
            }}

            func main() {{
                let s = ExcludedStruct(x: 1);
            }}
        "#
        ))
        .expect(HasError("ExcludedStruct"));
    }

    #[test]
    fn both_platforms_with_same_name_compiles() {
        // Both platforms define the same function — only the matching one should be included
        let current = if cfg!(target_os = "macos") {
            "darwin"
        } else {
            "linux"
        };
        let other = if cfg!(target_os = "macos") {
            "linux"
        } else {
            "darwin"
        };
        Test::new(&format!(
            r#"module Test
            @platform(.{current})
            func value() -> lang.i64 {{ 1 }}

            @platform(.{other})
            func value() -> lang.i64 {{ 2 }}
        "#
        ))
        .expect(Compiles);
    }
}

// =============================================================================
// ATTRIBUTE RECOGNITION
// =============================================================================

mod recognition {
    use super::*;

    #[test]
    fn platform_attribute_no_unknown_warning() {
        let platform = if cfg!(target_os = "macos") {
            "darwin"
        } else {
            "linux"
        };
        Test::new(&format!(
            r#"module Test
            @platform(.{platform})
            func foo() {{}}
        "#
        ))
        .expect(Compiles)
        .expect(NoWarnings);
    }

    #[test]
    fn platform_attribute_on_matching_has_attribute_behavior() {
        let platform = if cfg!(target_os = "macos") {
            "darwin"
        } else {
            "linux"
        };
        Test::new(&format!(
            r#"module Test
            @platform(.{platform})
            func foo() {{}}
        "#
        ))
        .expect(Compiles)
        .expect(
            Symbol::new("foo")
                .is(SymbolKind::Function)
                .has(Behavior::HasAttribute("platform")),
        );
    }
}
