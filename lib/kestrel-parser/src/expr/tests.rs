use super::*;
use crate::event::{EventSink, TreeBuilder};
use kestrel_lexer::lex;
use kestrel_span::Span;

fn parse_expr_from_source(source: &str) -> Expression {
    let tokens: Vec<_> = lex(source, 0)
        .filter_map(|t| t.ok())
        .map(|spanned| (spanned.value, spanned.span))
        .collect();

    let mut sink = EventSink::new();
    parse_expr(source, tokens.into_iter(), &mut sink);

    let tree = TreeBuilder::new(source, sink.into_events()).build();
    Expression {
        syntax: tree,
        span: Span::from(0..source.len()),
    }
}

// ===== Unit Expression Tests =====

#[test]
fn test_unit_expression() {
    let source = "()";
    let expr = parse_expr_from_source(source);

    assert!(expr.is_unit());
}

#[test]
fn test_unit_expression_with_whitespace() {
    let source = "  ()  ";
    let expr = parse_expr_from_source(source);

    assert!(expr.is_unit());
}

// ===== Integer Literal Tests =====

#[test]
fn test_integer_decimal() {
    let source = "42";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_integer());
}

#[test]
fn test_integer_hex() {
    let source = "0xFF";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_integer());
}

#[test]
fn test_integer_hex_uppercase() {
    let source = "0XAB";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_integer());
}

#[test]
fn test_integer_binary() {
    let source = "0b1010";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_integer());
}

#[test]
fn test_integer_octal() {
    let source = "0o755";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_integer());
}

// ===== Float Literal Tests =====

#[test]
fn test_float_simple() {
    let source = "3.14";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_float());
}

#[test]
fn test_float_scientific() {
    let source = "1.0e10";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_float());
}

#[test]
fn test_float_scientific_negative() {
    let source = "2.5E-3";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_float());
}

// ===== String Literal Tests =====

#[test]
fn test_string_simple() {
    let source = r#""hello""#;
    let expr = parse_expr_from_source(source);
    assert!(expr.is_string());
}

#[test]
fn test_string_with_escapes() {
    let source = r#""hello\nworld""#;
    let expr = parse_expr_from_source(source);
    assert!(expr.is_string());
}

#[test]
fn test_string_empty() {
    let source = r#""""#;
    let expr = parse_expr_from_source(source);
    assert!(expr.is_string());
}

// ===== Boolean Literal Tests =====

#[test]
fn test_bool_true() {
    let source = "true";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_bool());
}

#[test]
fn test_bool_false() {
    let source = "false";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_bool());
}

// ===== Array Literal Tests =====

#[test]
fn test_array_empty() {
    let source = "[]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_array());
}

#[test]
fn test_array_single() {
    let source = "[1]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_array());
}

#[test]
fn test_array_multiple() {
    let source = "[1, 2, 3]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_array());
}

#[test]
fn test_array_trailing_comma() {
    let source = "[1, 2, 3,]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_array());
}

#[test]
fn test_array_nested() {
    let source = "[[1, 2], [3, 4]]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_array());
}

#[test]
fn test_array_mixed_types() {
    let source = r#"[1, "hello", true]"#;
    let expr = parse_expr_from_source(source);
    assert!(expr.is_array());
}

// ===== Tuple Literal Tests =====

#[test]
fn test_tuple_single_element() {
    // Single element with trailing comma is a tuple
    let source = "(1,)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_tuple());
}

#[test]
fn test_tuple_two_elements() {
    let source = "(1, 2)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_tuple());
}

#[test]
fn test_tuple_multiple() {
    let source = "(1, 2, 3)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_tuple());
}

#[test]
fn test_tuple_trailing_comma() {
    let source = "(1, 2, 3,)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_tuple());
}

#[test]
fn test_tuple_nested() {
    let source = "((1, 2), (3, 4))";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_tuple());
}

// ===== Grouping Expression Tests =====

#[test]
fn test_grouping_integer() {
    // Single element without trailing comma is grouping
    let source = "(42)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_grouping());
}

#[test]
fn test_grouping_nested() {
    let source = "((42))";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_grouping());
}

#[test]
fn test_grouping_string() {
    let source = r#"("hello")"#;
    let expr = parse_expr_from_source(source);
    assert!(expr.is_grouping());
}

// ===== Mixed/Complex Tests =====

#[test]
fn test_array_of_tuples() {
    let source = "[(1, 2), (3, 4)]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_array());
}

#[test]
fn test_tuple_of_arrays() {
    let source = "([1, 2], [3, 4])";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_tuple());
}

#[test]
fn test_deeply_nested() {
    let source = "[[(1,)]]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_array());
}

// ===== Path Expression Tests =====

#[test]
fn test_path_single_segment() {
    let source = "foo";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_path());
}

#[test]
fn test_path_two_segments() {
    let source = "foo.bar";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_path());
}

#[test]
fn test_path_multiple_segments() {
    let source = "a.b.c.d";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_path());
}

#[test]
fn test_path_with_whitespace() {
    let source = "  foo . bar  ";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_path());
}

// ===== Unary Expression Tests =====

#[test]
fn test_unary_minus_integer() {
    let source = "-42";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_unary());
}

#[test]
fn test_unary_minus_float() {
    let source = "-3.14";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_unary());
}

#[test]
fn test_unary_bang() {
    let source = "!true";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_unary());
}

#[test]
fn test_unary_double_minus() {
    let source = "--42";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_unary());
}

#[test]
fn test_unary_double_bang() {
    let source = "!!false";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_unary());
}

#[test]
fn test_unary_minus_path() {
    let source = "-foo";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_unary());
}

#[test]
fn test_unary_minus_grouped() {
    let source = "-(1)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_unary());
}

// ===== Null Literal Tests =====

#[test]
fn test_null() {
    let source = "null";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_null());
}

#[test]
fn test_null_in_array() {
    let source = "[null, null]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_array());
}

#[test]
fn test_null_in_tuple() {
    let source = "(null, 42)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_tuple());
}

// ===== Call Expression Tests =====

#[test]
fn test_call_no_args() {
    let source = "foo()";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_call_single_arg() {
    let source = "foo(42)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_call_multiple_args() {
    let source = "foo(1, 2, 3)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_call_with_trailing_comma() {
    let source = "foo(1, 2,)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_call_labeled_arg() {
    let source = "foo(x: 42)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_call_mixed_labeled_unlabeled() {
    let source = "foo(1, name: \"test\", 3)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_call_chained() {
    let source = "foo()()";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_method_call() {
    let source = "obj.method()";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_method_call_with_args() {
    let source = "obj.method(1, 2)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_chained_method_calls() {
    let source = "a.b().c().d()";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_call_with_expression_args() {
    let source = "foo((1, 2), [3, 4])";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

// ===== Assignment Expression Tests =====

#[test]
fn test_assignment_simple() {
    let source = "x = 5";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_assignment());
}

#[test]
fn test_assignment_to_path() {
    let source = "point.x = 10";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_assignment());
}

#[test]
fn test_assignment_with_expression_rhs() {
    let source = "x = foo()";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_assignment());
}

#[test]
fn test_assignment_chain() {
    // a = b = c should parse as a = (b = c)
    let source = "a = b = c";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_assignment());
}

#[test]
fn test_assignment_with_complex_rhs() {
    let source = "result = obj.method(1, 2)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_assignment());
}

#[test]
fn test_assignment_with_array_rhs() {
    let source = "arr = [1, 2, 3]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_assignment());
}

#[test]
fn test_non_assignment_still_works() {
    // Verify that expressions without = still work
    let source = "foo.bar";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_path());
}

// ===== If Expression Tests =====

#[test]
fn test_if_without_else() {
    let source = "if true { 1 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_if());
}

#[test]
fn test_if_with_else() {
    let source = "if true { 1 } else { 2 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_if());
}

#[test]
fn test_if_else_if() {
    let source = "if a { 1 } else if b { 2 } else { 3 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_if());
}

#[test]
fn test_if_with_complex_condition() {
    let source = "if a and b { 1 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_if());
}

#[test]
fn test_if_with_statements_in_block() {
    let source = "if true { let x: Int = 1; x }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_if());
}

#[test]
fn test_nested_if() {
    let source = "if a { if b { 1 } else { 2 } } else { 3 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_if());
}

// ===== While Expression Tests =====

#[test]
fn test_while_basic() {
    let source = "while true { 1 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_while());
}

#[test]
fn test_while_with_condition() {
    let source = "while x > 0 { x }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_while());
}

#[test]
fn test_while_with_label() {
    let source = "outer: while true { 1 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_while());
}

// ===== Loop Expression Tests =====

#[test]
fn test_loop_basic() {
    let source = "loop { 1 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_loop());
}

#[test]
fn test_loop_with_label() {
    let source = "outer: loop { 1 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_loop());
}

// ===== Break Expression Tests =====

#[test]
fn test_break_simple() {
    let source = "break";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_break());
}

#[test]
fn test_break_with_label() {
    let source = "break outer";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_break());
}

// ===== Continue Expression Tests =====

#[test]
fn test_continue_simple() {
    let source = "continue";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_continue());
}

#[test]
fn test_continue_with_label() {
    let source = "continue outer";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_continue());
}

// ===== Closure Expression Tests =====

#[test]
fn test_closure_no_params_no_body() {
    let source = "{ }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_closure());
}

#[test]
fn test_closure_no_params_simple_body() {
    let source = "{ 42 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_closure());
}

#[test]
fn test_closure_with_single_param() {
    let source = "{ (x) in x * 2 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_closure());
}

#[test]
fn test_closure_with_multiple_params() {
    let source = "{ (x, y) in x + y }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_closure());
}

#[test]
fn test_closure_with_type_annotation() {
    let source = "{ (x: Int) in x * 2 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_closure());
}

#[test]
fn test_closure_with_mixed_type_annotations() {
    let source = "{ (x: Int, y) in x + y }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_closure());
}

#[test]
fn test_closure_with_statements() {
    let source = "{ (x) in let y: Int = x * 2; y }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_closure());
}

#[test]
fn test_closure_empty_params() {
    let source = "{ () in 42 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_closure());
}

// ===== Type Arguments in Expression Tests =====

#[test]
fn test_path_with_type_args() {
    let source = "List[Int]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_path());
}

#[test]
fn test_path_with_multiple_type_args() {
    let source = "Map[String, Int]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_path());
}

#[test]
fn test_path_with_nested_type_args() {
    let source = "List[Option[Int]]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_path());
}

#[test]
fn test_call_with_type_args() {
    let source = "foo[Int]()";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_call_with_type_args_and_args() {
    let source = "helper[String](x)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_call_with_multiple_type_args() {
    let source = "convert[Int, String](42)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_method_call_with_type_args() {
    let source = "obj.method[Int]()";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_chained_path_with_type_args() {
    let source = "Container[Int].Nested[String]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_path());
}

#[test]
fn test_static_method_with_type_args() {
    let source = "Container[Int].create()";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_path_type_args_then_method() {
    let source = "List[Int].new().push(1)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

// ===== Trailing Closure Tests =====

#[test]
fn test_trailing_closure_simple() {
    // apply { 42 }
    let source = "apply { 42 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_trailing_closure_with_params() {
    // apply { (x) in x * 2 }
    let source = "apply { (x) in x * 2 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_trailing_closure_after_parens() {
    // fold(0) { (acc, n) in acc + n }
    let source = "fold(0) { (acc, n) in acc + n }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_trailing_closure_multiple_args() {
    // combine(1, 2) { it * 2 }
    let source = "combine(1, 2) { it * 2 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_trailing_closure_with_label() {
    // apply f: { 42 }
    let source = "apply f: { 42 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_trailing_closure_on_method() {
    // obj.method { 42 }
    let source = "obj.method { 42 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_trailing_closure_chained() {
    // foo().bar { 42 }
    let source = "foo().bar { 42 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_multiple_trailing_closures() {
    // configure onTap: { 1 } onLongPress: { 2 }
    let source = "configure onTap: { 1 } onLongPress: { 2 }";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

#[test]
fn test_trailing_closure_after_args_with_label() {
    // Button(title: "OK") onTap: { save() }
    let source = r#"Button(title: "OK") onTap: { save() }"#;
    let expr = parse_expr_from_source(source);
    assert!(expr.is_call());
}

// ===== Implicit Member Access Tests =====

#[test]
fn test_implicit_member_access_simple() {
    // .Foo - simple implicit member access for enum case
    let source = ".Foo";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_implicit_member_access());
}

#[test]
fn test_implicit_member_access_with_empty_args() {
    // .Foo() - with empty argument list
    let source = ".Foo()";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_implicit_member_access());
}

#[test]
fn test_implicit_member_access_with_labeled_arg() {
    // .Foo(x: 1) - with labeled argument
    let source = ".Foo(x: 1)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_implicit_member_access());
}

#[test]
fn test_implicit_member_access_with_multiple_args() {
    // .Foo(x: 1, y: 2) - with multiple labeled arguments
    let source = ".Foo(x: 1, y: 2)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_implicit_member_access());
}

#[test]
fn test_implicit_member_access_with_unlabeled_arg() {
    // .Some(42) - with unlabeled argument (like Option.Some)
    let source = ".Some(42)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_implicit_member_access());
}

#[test]
fn test_implicit_member_access_with_expression_arg() {
    // .Point(x: 1 + 2, y: 3 * 4) - with complex expression arguments
    let source = ".Point(x: 1 + 2, y: 3 * 4)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_implicit_member_access());
}

#[test]
fn test_implicit_member_access_in_array() {
    // [.north, .south] - implicit member access in array literal
    let source = "[.north, .south]";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_array());
}

#[test]
fn test_implicit_member_access_with_trailing_comma() {
    // .Foo(x: 1,) - with trailing comma in arguments
    let source = ".Foo(x: 1,)";
    let expr = parse_expr_from_source(source);
    assert!(expr.is_implicit_member_access());
}


