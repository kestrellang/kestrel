//! Type checking validation tests
//!
//! These tests verify that the type checker catches type mismatches.

use kestrel_test_suite::*;

mod return_types {
    use super::*;

    #[test]
    fn return_wrong_type() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    return "hello"
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn return_int_for_string() {
        Test::new(
            r#"
module Main

func test() -> lang.str {
    return 42
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn return_bool_for_int() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    return true
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn bare_return_in_non_unit_function() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    return
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn bare_return_in_unit_function_ok() {
        Test::new(
            r#"
module Main

func test() {
    return
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn return_unit_for_int() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    return ()
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn implicit_return_wrong_type() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    "not an lang.i64"
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn return_tuple_for_int() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    return (1, 2)
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn return_array_for_int() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    return [1, 2, 3]
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn correct_return_type_ok() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    return 42
}
"#,
        )
        .expect(Compiles);
    }
}

mod assignment_types {
    use super::*;

    #[test]
    fn assign_string_to_int_variable() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    x = "hello"
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn assign_bool_to_string_variable() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.str = "hello";
    x = true
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn assign_int_to_bool_variable() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i1 = true;
    x = 42
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn assign_tuple_to_int_variable() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    x = (1, 2)
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn assign_correct_type_ok() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    x = 42
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn assign_to_struct_field_wrong_type() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() {
    var p: Point = Point(x: 0, y: 0);
    p.x = "not an lang.i64"
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }
}

mod variable_binding_types {
    use super::*;

    #[test]
    fn bind_string_to_int_variable() {
        Test::new(
            r#"
module Main

func test() {
    let x: lang.i64 = "hello";
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn bind_bool_to_string_variable() {
        Test::new(
            r#"
module Main

func test() {
    let x: lang.str = false;
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn bind_int_to_bool_variable() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i1 = 123;
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn bind_tuple_to_int_variable() {
        Test::new(
            r#"
module Main

func test() {
    let x: lang.i64 = (1, 2, 3);
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn bind_array_to_int_variable() {
        Test::new(
            r#"
module Main

func test() {
    let x: lang.i64 = [1, 2, 3];
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn bind_correct_type_ok() {
        Test::new(
            r#"
module Main

func test() {
    let x: lang.i64 = 42;
    let y: lang.str = "hello";
    let z: lang.i1 = true;
}
"#,
        )
        .expect(Compiles);
    }
}

mod condition_types {
    use super::*;

    #[test]
    fn if_condition_int() {
        Test::new(
            r#"
module Main

func test() {
    if 42 {
        let x: lang.i64 = 1;
    }
}
"#,
        )
        .expect(HasError("must conform to `BooleanConditional`"));
    }

    #[test]
    fn if_condition_string() {
        Test::new(
            r#"
module Main

func test() {
    if "hello" {
        let x: lang.i64 = 1;
    }
}
"#,
        )
        .expect(HasError("must conform to `BooleanConditional`"));
    }

    #[test]
    fn while_condition_int() {
        Test::new(
            r#"
module Main

func test() {
    while 42 {
        let x: lang.i64 = 1;
    }
}
"#,
        )
        .expect(HasError("must conform to `BooleanConditional`"));
    }

    #[test]
    fn while_condition_string() {
        Test::new(
            r#"
module Main

func test() {
    while "hello" {
        let x: lang.i64 = 1;
    }
}
"#,
        )
        .expect(HasError("must conform to `BooleanConditional`"));
    }

    #[test]
    fn if_condition_tuple() {
        // Use a variable to avoid parsing ambiguity with if (...)
        Test::new(
            r#"
module Main

func test() {
    let t: (lang.i1, lang.i1) = (true, false);
    if t {
        let x: lang.i64 = 1;
    }
}
"#,
        )
        .expect(HasError("must conform to `BooleanConditional`"));
    }

    #[test]
    fn if_condition_unit() {
        // Use a variable to avoid parsing ambiguity
        Test::new(
            r#"
module Main

func test() {
    let u: () = ();
    if u {
        let x: lang.i64 = 1;
    }
}
"#,
        )
        .expect(HasError("must conform to `BooleanConditional`"));
    }

    #[test]
    fn if_condition_bool_ok() {
        Test::new(
            r#"
module Main

func test() {
    if true {
        let x: lang.i64 = 1;
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn while_condition_bool_ok() {
        Test::new(
            r#"
module Main

func test() {
    while false {
        let x: lang.i64 = 1;
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn if_condition_comparison_ok() {
        Test::new(
            r#"
module Main

func test() {
    let x: lang.i64 = 5;
    if lang.i64_eq(x, 5) {
        let y: lang.i64 = 1;
    }
}
"#,
        )
        .expect(Compiles);
    }
}

mod branch_types {
    use super::*;

    #[test]
    fn if_else_branches_mismatch_int_string() {
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        42
    } else {
        "hello"
    }
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn if_else_branches_mismatch_bool_int() {
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        true
    } else {
        42
    }
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn if_else_branches_mismatch_tuple_int() {
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        (1, 2)
    } else {
        42
    }
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn if_else_if_else_branches_mismatch() {
        Test::new(
            r#"
module Main

func test(x: lang.i64) -> lang.i64 {
    if lang.i64_eq(x, 1) {
        10
    } else if lang.i64_eq(x, 2) {
        "twenty"
    } else {
        30
    }
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn if_else_branches_same_type_ok() {
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        42
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn if_without_else_unit_ok() {
        // Without else, if has type Unit - no branch mismatch
        Test::new(
            r#"
module Main

func test(cond: lang.i1) {
    if cond {
        let x: lang.i64 = 42;
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_if_else_mismatch() {
        Test::new(
            r#"
module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    if a {
        if b {
            42
        } else {
            "wrong"
        }
    } else {
        0
    }
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }
}

mod call_argument_types {
    use super::*;

    #[test]
    fn call_with_wrong_arg_type() {
        Test::new(
            r#"
module Main

func greet(name: lang.str) {}

func test() {
    greet(42)
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn call_with_wrong_second_arg() {
        Test::new(
            r#"
module Main

func add(a: lang.i64, b: lang.i64) -> lang.i64 {
    lang.i64_add(a, b)
}

func test() {
    add(1, "two")
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn call_with_wrong_first_arg() {
        Test::new(
            r#"
module Main

func add(a: lang.i64, b: lang.i64) -> lang.i64 {
    lang.i64_add(a, b)
}

func test() {
    add("one", 2)
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn call_with_tuple_for_int() {
        Test::new(
            r#"
module Main

func double(x: lang.i64) -> lang.i64 {
    lang.i64_add(x, x)
}

func test() {
    double((1, 2))
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn call_with_correct_args_ok() {
        Test::new(
            r#"
module Main

func add(a: lang.i64, b: lang.i64) -> lang.i64 {
    lang.i64_add(a, b)
}

func test() -> lang.i64 {
    add(1, 2)
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn call_non_callable_int() {
        // TODO: Report error: trying to call non-callable
        Test::new(
            r#"
module Main
func test() {
    let x = 42;
    x()
}
"#,
        )
        .expect(HasError("is not callable"));
    }

    #[test]
    fn call_non_callable_struct() {
        // TODO: Report error: expression is not callable
        Test::new(
            r#"
module Main
struct S { }
func test() {
    let s = S();
    s()
}
"#,
        )
        .expect(HasError("is not callable"));
    }

    #[test]
    fn method_call_with_wrong_arg() {
        Test::new(
            r#"
module Main

struct Calculator {
    var value: lang.i64

    func add(x: lang.i64) -> lang.i64 {
        lang.i64_add(self.value, x)
    }
}

func test() {
    let calc: Calculator = Calculator(value: 10);
    calc.add("five")
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }
}

mod struct_init_types {
    use super::*;

    #[test]
    fn struct_init_wrong_field_type() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() {
    let p: Point = Point(x: "zero", y: 0);
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn struct_init_wrong_second_field_type() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() {
    let p: Point = Point(x: 0, y: "zero");
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn struct_init_all_fields_wrong() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() {
    let p: Point = Point(x: "a", y: "b");
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn struct_init_correct_types_ok() {
        Test::new(
            r#"
module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() {
    let p: Point = Point(x: 10, y: 20);
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_init_bool_for_int() {
        Test::new(
            r#"
module Main

struct Config {
    var count: lang.i64
    var enabled: lang.i1
}

func test() {
    let c: Config = Config(count: true, enabled: 42);
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }
}

mod array_element_types {
    use super::*;

    #[test]
    fn array_mixed_int_string() {
        Test::new(
            r#"
module Main

func test() {
    let arr: [lang.i64] = [1, "two", 3];
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn array_mixed_int_bool() {
        Test::new(
            r#"
module Main

func test() {
    let arr: [lang.i64] = [1, 2, true];
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn array_mixed_string_int() {
        Test::new(
            r#"
module Main

func test() {
    let arr: [lang.str] = ["hello", 42];
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn array_all_same_type_ok() {
        Test::new(
            r#"
module Main

func test() {
    let ints: [lang.i64] = [1, 2, 3];
    let strings: [lang.str] = ["a", "b", "c"];
    let bools: [lang.i1] = [true, false, true];
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn array_single_element_ok() {
        Test::new(
            r#"
module Main

func test() {
    let arr: [lang.i64] = [42];
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn array_mixed_multiple_wrong() {
        // Multiple type errors in one array
        Test::new(
            r#"
module Main

func test() {
    let arr: [lang.i64] = [1, "two", true, 4.0];
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }
}

mod never_type {
    use super::*;

    #[test]
    fn never_in_if_else_ok() {
        // Never type propagates correctly
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        42
    } else {
        return 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn never_in_both_branches_ok() {
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        return 1
    } else {
        return 2
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn break_propagates_never() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    loop {
        if true {
            break
        } else {
            return 42
        }
    }
    0
}
"#,
        )
        .expect(Compiles);
    }
}

mod struct_types {
    use super::*;

    // TODO: These tests are disabled because struct type comparison by SymbolId
    // is not working correctly when structs are from different type resolution paths.
    // This needs investigation - the struct types should have different IDs but
    // they're being treated as compatible.

    #[test]
    fn assign_different_struct_types() {
        Test::new(
            r#"
module Main

struct Point { var x: lang.i64; var y: lang.i64 }
struct Size { var width: lang.i64; var height: lang.i64 }

func test() {
    var p: Point = Point(x: 0, y: 0);
    let s: Size = Size(width: 10, height: 20);
    p = s
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn return_wrong_struct_type() {
        Test::new(
            r#"
module Main

struct Point { var x: lang.i64; var y: lang.i64 }
struct Size { var width: lang.i64; var height: lang.i64 }

func makePoint() -> Point {
    Size(width: 10, height: 20)
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn pass_wrong_struct_to_function() {
        Test::new(
            r#"
module Main

struct Point { var x: lang.i64; var y: lang.i64 }
struct Size { var width: lang.i64; var height: lang.i64 }

func usePoint(p: Point) {}

func test() {
    usePoint(Size(width: 10, height: 20))
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn same_struct_type_ok() {
        Test::new(
            r#"
module Main

struct Point { var x: lang.i64; var y: lang.i64 }

func test() {
    var p1: Point = Point(x: 0, y: 0);
    let p2: Point = Point(x: 1, y: 1);
    p1 = p2
}
"#,
        )
        .expect(Compiles);
    }
}

mod tuple_types {
    use super::*;

    #[test]
    fn tuple_wrong_element_count() {
        Test::new(
            r#"
module Main

func test() {
    let t: (lang.i64, lang.i64) = (1, 2, 3);
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn tuple_wrong_element_types() {
        Test::new(
            r#"
module Main

func test() {
    let t: (lang.i64, lang.str) = (1, 2);
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn tuple_correct_ok() {
        Test::new(
            r#"
module Main

func test() {
    let t: (lang.i64, lang.str, lang.i1) = (42, "hello", true);
}
"#,
        )
        .expect(Compiles);
    }
}

mod function_types {
    use super::*;

    // Note: We can't easily test function type mismatches yet since
    // we don't have closures or function references fully implemented.
    // These tests are placeholders for future features.

    #[test]
    fn function_returning_function_ok() {
        // Functions with function return types should compile
        // when returning the right type
        Test::new(
            r#"
module Main

func identity(x: lang.i64) -> lang.i64 { x }
"#,
        )
        .expect(Compiles);
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn deeply_nested_type_mismatch() {
        Test::new(
            r#"
module Main

func test(a: lang.i1, b: lang.i1, c: lang.i1) -> lang.i64 {
    if a {
        if b {
            if c {
                42
            } else {
                "wrong"
            }
        } else {
            0
        }
    } else {
        0
    }
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn chained_assignments_type_mismatch() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    var y: lang.str = "hello";
    x = 42;
    y = x
}
"#,
        )
        .expect(HasError("type mismatch"));
    }

    #[test]
    fn return_from_nested_if() {
        Test::new(
            r#"
module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    if a {
        if b {
            return "wrong"
        }
        1
    } else {
        2
    }
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn while_with_wrong_return() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    while true {
        return "not an lang.i64"
    }
    0
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn loop_with_wrong_return() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    loop {
        return "not an lang.i64"
    }
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn multiple_errors_in_function() {
        // Should catch at least one error
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let x: lang.i64 = "wrong1";
    let y: lang.str = 42;
    return true
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }
}

mod type_alias {
    use super::*;

    #[test]
    fn type_alias_expanded_correctly() {
        // Type aliases should be expanded for comparison
        Test::new(
            r#"
module Main

type MyInt = lang.i64;

func test() {
    let x: MyInt = 42;
    let y: lang.i64 = x;
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn type_alias_mismatch() {
        Test::new(
            r#"
module Main

type MyInt = lang.i64;

func test() {
    let x: MyInt = "not an lang.i64";
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn chained_type_alias() {
        Test::new(
            r#"
module Main

type MyInt = lang.i64;
type YourInt = MyInt;

func test() {
    let x: YourInt = 42;
    let y: lang.i64 = x;
}
"#,
        )
        .expect(Compiles);
    }
}

mod tuple_indexing {
    use super::*;

    #[test]
    fn basic_tuple_index() {
        Test::new(
            r#"
module Main

func test() {
    let t = (1, "hello", true);
    let x: lang.i64 = t.0;
    let y: lang.str = t.1;
    let z: lang.i1 = t.2;
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_index_wrong_type() {
        Test::new(
            r#"
module Main

func test() {
    let t = (1, "hello");
    let x: lang.str = t.0;
}
"#,
        )
        .expect(HasError("does not conform to protocol"));
    }

    #[test]
    fn tuple_index_out_of_bounds() {
        Test::new(
            r#"
module Main

func test() {
    let t = (1, 2);
    let x = t.5;
}
"#,
        )
        .expect(HasError("out of bounds"));
    }

    #[test]
    fn tuple_index_on_non_tuple() {
        Test::new(
            r#"
module Main

func test() {
    let x = 42;
    let y = x.0;
}
"#,
        )
        .expect(HasError("cannot use tuple index"));
    }

    #[test]
    fn chained_tuple_index() {
        // Note: t.0.1 is currently parsed as t.0 (tuple index) then .1 (float literal .1)
        // due to lexer ambiguity. We work around this by using intermediate variables.
        Test::new(
            r#"
module Main

func test() {
    let t = ((1, 2), (3, 4));
    let inner = t.0;
    let x: lang.i64 = inner.1;
    let inner2 = t.1;
    let y: lang.i64 = inner2.0;
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_index_mutability() {
        Test::new(
            r#"
module Main

func test() {
    var t = (1, 2);
    t.0 = 10;
    t.1 = 20;
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_index_immutable_error() {
        Test::new(
            r#"
module Main

func test() {
    let t = (1, 2);
    t.0 = 10;
}
"#,
        )
        .expect(HasError("cannot assign"));
    }

    #[test]
    fn tuple_index_from_function_return() {
        Test::new(
            r#"
module Main

func getTuple() -> (lang.i64, lang.str) {
    return (42, "hello");
}

func test() {
    let x: lang.i64 = getTuple().0;
    let y: lang.str = getTuple().1;
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn large_tuple_index() {
        Test::new(
            r#"
module Main

func test() {
    let t = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
    let x: lang.i64 = t.9;
}
"#,
        )
        .expect(Compiles);
    }
}
