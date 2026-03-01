//! Tuple codegen tests.

use kestrel_test_suite::*;

#[test]
fn test_tuple_construction() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let t = (42, 0);
    if t.0 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_second_element() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let t = (0, 42);
    if t.1 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_multiple_elements() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let t: (std.num.Int64, std.num.Int64, std.num.Int64) = (10, 20, 12);
    if t.0 + t.1 + t.2 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_mixed_types() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let t = (true, 42);
    if t.1 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_nested_tuple() {
    // Note: t.0.0 parses as t.(0.0) which is a float literal
    // So we use a temporary variable to work around this parser limitation
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let t: ((std.num.Int64, std.num.Int64), std.num.Int64) = ((40, 2), 0);
    let inner = t.0;
    if inner.0 + inner.1 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_field_member_access() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let pair = ("hello", "world");
    if pair.0.byteCount != 5 { return 1 }
    if pair.1.byteCount != 5 { return 2 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_field_method_call() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let pair = ("hello", "world");
    if pair.0.equals("hello") != true { return 1 }
    if pair.1.equals("hello") != false { return 2 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_field_chained_operations() {
    Test::new(
        r#"module Test

func main() -> lang.i64 {
    let pair = ("hello world", 42);
    let len = pair.0.byteCount;
    if len != 11 { return 1 }
    if pair.1 != 42 { return 2 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_field_method_in_loop() {
    // Exercises the pattern used in HTTP header lookup:
    // iterating an array of tuples, calling a method on a tuple field
    Test::new(
        r#"module Test

func findValue(pairs: Array[(String, String)], name: String) -> String? {
    var i: Int64 = 0;
    while i < pairs.count {
        let pair = pairs(unchecked: i);
        if pair.0.equals(name) {
            return .Some(pair.1)
        }
        i = i + 1
    }
    .None
}

func main() -> lang.i64 {
    var headers = Array[(String, String)]();
    headers.append(("Content-Type", "text/html"));
    headers.append(("Host", "example.com"));
    headers.append(("Accept", "application/json"));

    let ct = findValue(headers, "Content-Type");
    match ct {
        .Some(v) => if v.equals("text/html") == false { return 1 },
        .None => return 2
    }

    let host = findValue(headers, "Host");
    match host {
        .Some(v) => if v.equals("example.com") == false { return 3 },
        .None => return 4
    }

    let missing = findValue(headers, "X-Missing");
    match missing {
        .Some(_) => return 5,
        .None => 0
    }

    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_field_as_function_arg() {
    // Exercises the pattern used in file seek:
    // passing tuple fields directly as function arguments
    Test::new(
        r#"module Test

func add(a: Int64, b: Int64) -> Int64 {
    a + b
}

func main() -> lang.i64 {
    let pair = (20, 22);
    let result = add(pair.0, pair.1);
    if result != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn test_tuple_from_function() {
    Test::new(
        r#"module Test

func make_pair(a: std.num.Int64, b: std.num.Int64) -> (std.num.Int64, std.num.Int64) {
    (a, b)
}

func main() -> lang.i64 {
    let t = make_pair(20, 22);
    if t.0 + t.1 != 42 { return 1 }
    0
}
"#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
