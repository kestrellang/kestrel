//! Intended-behavior tests for global and struct properties.
//!
//! These tests encode the desired semantics from examples/properties/*.ks.
//! They are expected to fail with the current implementation until the
//! missing behaviors are implemented.

use kestrel_test_suite::*;

// =============================================================================
// Global properties (module scope)
// =============================================================================

mod global_properties {
    use super::*;

    #[test]
    fn global_let_initial_value() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

public let globalLet: std.num.Int64 = 7;

func main() -> std.num.Int64 {
    let _ = println(globalLet);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("7\n"));
    }

    #[test]
    fn global_var_mutability_and_initial_value() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

public var globalVar: std.num.Int64 = 0;

func main() -> std.num.Int64 {
    let _ = println(globalVar);
    globalVar = 5;
    let _ = println(globalVar);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("0\n5\n"));
    }

    #[test]
    fn global_computed_var_get_set() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

private var _g: std.num.Int64 = 1;

public var globalComputedVar: std.num.Int64 {
    get { _g }
    set { _g = newValue }
}

func main() -> std.num.Int64 {
    let _ = println(globalComputedVar);
    globalComputedVar = 2;
    let _ = println(globalComputedVar);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("1\n2\n"));
    }

    #[test]
    fn global_static_let_disallowed() {
        Test::new(
            r#"
module Test
public static let globalStaticLet: std.num.Int64 = 0;
"#,
        )
        .with_stdlib()
        .expect(HasError("properties in global context are already static"));
    }

    #[test]
    fn global_static_var_disallowed() {
        Test::new(
            r#"
module Test
public static var globalStaticVar: std.num.Int64 = 0;
"#,
        )
        .with_stdlib()
        .expect(HasError("properties in global context are already static"));
    }

    #[test]
    fn global_computed_let_disallowed() {
        Test::new(
            r#"
module Test
public let globalComputedLet: std.num.Int64 { 0 }
"#,
        )
        .with_stdlib()
        .expect(HasError("computed properties must use 'var'"));
    }

    #[test]
    fn global_static_computed_let_disallowed() {
        Test::new(
            r#"
module Test
public static let globalStaticComputedLet: std.num.Int64 { 0 }
"#,
        )
        .with_stdlib()
        .expect(HasError("computed properties must use 'var'"));
    }

    #[test]
    fn global_static_computed_var_disallowed() {
        Test::new(
            r#"
module Test
public static var globalStaticComputedVar: std.num.Int64 { 0 }
"#,
        )
        .with_stdlib()
        .expect(HasError("computed properties in global context are already static"));
    }
}

// =============================================================================
// Struct properties
// =============================================================================

mod struct_properties {
    use super::*;

    #[test]
    fn struct_let_initial_value() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

public struct Foo {
    public let structLet: std.num.Int64 = 11;
}

func main() -> std.num.Int64 {
    let foo = Foo(structLet: 11);
    let _ = println(foo.structLet);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("11\n"));
    }

    #[test]
    fn struct_var_mutability_and_initial_value() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

public struct Foo {
    public var structVar: std.num.Int64 = 0;
}

func main() -> std.num.Int64 {
    var foo = Foo(structVar: 0);
    let _ = println(foo.structVar);
    foo.structVar = 3;
    let _ = println(foo.structVar);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("0\n3\n"));
    }

    #[test]
    fn struct_static_let_initial_value() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

public struct Foo {
    public static let structStaticLet: std.num.Int64 = 10;
}

func main() -> std.num.Int64 {
    let _ = println(Foo.structStaticLet);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("10\n"));
    }

    #[test]
    fn struct_static_var_mutability_and_initial_value() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

public struct Foo {
    public static var structStaticVar: std.num.Int64 = 1;
}

func main() -> std.num.Int64 {
    let _ = println(Foo.structStaticVar);
    Foo.structStaticVar = 2;
    let _ = println(Foo.structStaticVar);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("1\n2\n"));
    }

    #[test]
    fn struct_computed_var_get_set() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

public struct Foo {
    private var _v: std.num.Int64

    public var structComputedVar: std.num.Int64 {
        get { self._v }
        set { self._v = newValue }
    }

    init() { self._v = 5 }
}

func main() -> std.num.Int64 {
    var foo = Foo();
    let _ = println(foo.structComputedVar);
    foo.structComputedVar = 9;
    let _ = println(foo.structComputedVar);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("5\n9\n"));
    }

    #[test]
    fn struct_static_computed_var_get_set() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

public struct Foo {
    private static var _s: std.num.Int64 = 5;

    public static var structStaticComputedVar: std.num.Int64 {
        get { _s }
        set { _s = newValue }
    }
}

func main() -> std.num.Int64 {
    let _ = println(Foo.structStaticComputedVar);
    Foo.structStaticComputedVar = 7;
    let _ = println(Foo.structStaticComputedVar);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("5\n7\n"));
    }

    #[test]
    fn struct_computed_let_disallowed() {
        Test::new(
            r#"
module Test
public struct Foo {
    public let structComputedLet: std.num.Int64 { 0 }
}
"#,
        )
        .with_stdlib()
        .expect(HasError("computed properties must use 'var'"));
    }

    #[test]
    fn struct_static_computed_let_disallowed() {
        Test::new(
            r#"
module Test
public struct Foo {
    public static let structStaticComputedLet: std.num.Int64 { 0 }
}
"#,
        )
        .with_stdlib()
        .expect(HasError("computed properties must use 'var'"));
    }

    #[test]
    fn struct_non_static_let_disallowed() {
        // This test verifies that instance let fields ARE allowed in structs
        // (unlike enums). This should compile successfully.
        Test::new(
            r#"
module Main
public struct Foo {
    public let x: std.num.Int64
}
func main() -> std.num.Int64 { 0 }
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn struct_non_static_var_disallowed() {
        // This test verifies that instance var fields ARE allowed in structs
        // (unlike enums). This should compile successfully.
        Test::new(
            r#"
module Main
public struct Foo {
    public var x: std.num.Int64
}
func main() -> std.num.Int64 { 0 }
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// =============================================================================
// Enum properties (intended behavior: no instance stored fields; static stored ok)
// =============================================================================

mod enum_properties {
    use super::*;

    #[test]
    fn enum_non_static_let_disallowed() {
        Test::new(
            r#"
module Test
enum Foo {
    case A
    let x: std.num.Int64
}
"#,
        )
        .with_stdlib()
        .expect(HasError("enums cannot have stored fields"));
    }

    #[test]
    fn enum_non_static_var_disallowed() {
        Test::new(
            r#"
module Test
enum Foo {
    case A
    var x: std.num.Int64
}
"#,
        )
        .with_stdlib()
        .expect(HasError("enums cannot have stored fields"));
    }

    #[test]
    fn enum_static_let_initial_value() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

enum Foo {
    case A
    static let staticLet: std.num.Int64 = 4;
}

func main() -> std.num.Int64 {
    let _ = println(Foo.staticLet);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("4\n"));
    }

    #[test]
    fn enum_static_var_mutability_and_initial_value() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

enum Foo {
    case A
    static var staticVar: std.num.Int64 = 1;
}

func main() -> std.num.Int64 {
    let _ = println(Foo.staticVar);
    Foo.staticVar = 2;
    let _ = println(Foo.staticVar);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("1\n2\n"));
    }

    #[test]
    fn enum_computed_let_disallowed() {
        Test::new(
            r#"
module Test
enum Foo {
    case A
    let computed: std.num.Int64 { 0 }
}
"#,
        )
        .with_stdlib()
        .expect(HasError("computed properties must use 'var'"));
    }

    #[test]
    fn enum_computed_var_get_set() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

enum Foo {
    case A
    private static var _v: std.num.Int64 = 1;

    var computed: std.num.Int64 {
        get { Foo._v }
        set { Foo._v = newValue }
    }
}

func main() -> std.num.Int64 {
    var f: Foo = .A;
    let _ = println(f.computed);
    f.computed = 3;
    let _ = println(f.computed);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("1\n3\n"));
    }

    #[test]
    fn enum_static_computed_let_disallowed() {
        Test::new(
            r#"
module Test
enum Foo {
    case A
    static let computed: std.num.Int64 { 0 }
}
"#,
        )
        .with_stdlib()
        .expect(HasError("computed properties must use 'var'"));
    }

    #[test]
    fn enum_static_computed_var_get_set() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

enum Foo {
    case A
    private static var _s: std.num.Int64 = 5;

    static var computed: std.num.Int64 {
        get { _s }
        set { _s = newValue }
    }
}

func main() -> std.num.Int64 {
    let _ = println(Foo.computed);
    Foo.computed = 7;
    let _ = println(Foo.computed);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("5\n7\n"));
    }
}

// =============================================================================
// Protocol properties (intended behavior)
// =============================================================================

mod protocol_properties {
    use super::*;

    #[test]
    fn protocol_non_static_let_requirement_type_mismatch() {
        Test::new(
            r#"
module Test

protocol P {
    let value: std.num.Int64
}

struct S: P {
    let value: std.num.Int32
}
"#,
        )
        .with_stdlib()
        .expect(HasError("property 'value' has wrong type for protocol"));
    }

    #[test]
    fn protocol_non_static_var_requirement_type_mismatch() {
        Test::new(
            r#"
module Test

protocol P {
    var value: std.num.Int64
}

struct S: P {
    var value: std.num.Int32
}
"#,
        )
        .with_stdlib()
        .expect(HasError("property 'value' has wrong type for protocol"));
    }

    #[test]
    fn protocol_static_let_requirement_type_mismatch() {
        Test::new(
            r#"
module Test

protocol P {
    static let value: std.num.Int64
}

struct S: P {
    static let value: std.num.Int32 = 0
}
"#,
        )
        .with_stdlib()
        .expect(HasError("property 'value' has wrong type for protocol"));
    }

    #[test]
    fn protocol_static_var_requirement_type_mismatch() {
        Test::new(
            r#"
module Test

protocol P {
    static var value: std.num.Int64
}

struct S: P {
    static var value: std.num.Int32 = 0
}
"#,
        )
        .with_stdlib()
        .expect(HasError("property 'value' has wrong type for protocol"));
    }

    #[test]
    fn protocol_non_static_computed_let_disallowed() {
        Test::new(
            r#"
module Test

protocol P {
    let value: std.num.Int64 { get }
}
"#,
        )
        .with_stdlib()
        .expect(HasError("computed properties must use 'var'"));
    }

    #[test]
    fn protocol_non_static_computed_var_requirement_type_mismatch() {
        Test::new(
            r#"
module Test

protocol P {
    var value: std.num.Int64 { get }
}

struct S: P {
    var value: std.num.Int32 { 0 }
}
"#,
        )
        .with_stdlib()
        .expect(HasError("property 'value' has wrong type for protocol"));
    }

    #[test]
    fn protocol_static_computed_let_disallowed() {
        Test::new(
            r#"
module Test

protocol P {
    static let value: std.num.Int64 { get }
}
"#,
        )
        .with_stdlib()
        .expect(HasError("computed properties must use 'var'"));
    }

    #[test]
    fn protocol_static_computed_var_requirement_type_mismatch() {
        Test::new(
            r#"
module Test

protocol P {
    static var value: std.num.Int64 { get }
}

struct S: P {
    static var value: std.num.Int32 { 0 }
}
"#,
        )
        .with_stdlib()
        .expect(HasError("property 'value' has wrong type for protocol"));
    }
}
