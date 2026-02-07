use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn expression_bodied_function() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func answer() -> std.num.Int64 = 42

func main() -> std.num.Int64 {
    let _ = println(answer());
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("42\n"));
    }

    #[test]
    fn expression_bodied_function_with_parameters() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func add(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 = a + b

func main() -> std.num.Int64 {
    let _ = println(add(3, 4));
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("7\n"));
    }

    #[test]
    fn expression_bodied_function_unit_return() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func doNothing() -> () = ()

func main() -> std.num.Int64 {
    doNothing();
    let _ = println(1);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("1\n"));
    }

    #[test]
    fn expression_bodied_public_function() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

public func publicAnswer() -> std.num.Int64 = 99

func main() -> std.num.Int64 {
    let _ = println(publicAnswer());
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("99\n"));
    }
}

mod methods {
    use super::*;

    #[test]
    fn expression_bodied_instance_method() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64

    func sum() -> std.num.Int64 = self.x + self.y
}

func main() -> std.num.Int64 {
    let p = Point(x: 3, y: 4);
    let _ = println(p.sum());
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("7\n"));
    }

    #[test]
    fn expression_bodied_static_method() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

struct Factory {
    static func create() -> std.num.Int64 = 42
}

func main() -> std.num.Int64 {
    let _ = println(Factory.create());
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("42\n"));
    }

    #[test]
    fn expression_bodied_mutating_method() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

struct Counter {
    var count: std.num.Int64

    mutating func reset() -> () = self.count = 0
}

func main() -> std.num.Int64 {
    var c = Counter(count: 10);
    c.reset();
    let _ = println(c.count);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("0\n"));
    }
}

mod generics {
    use super::*;

    #[test]
    fn expression_bodied_generic_function() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func identity[T](x: T) -> T = x

func main() -> std.num.Int64 {
    let _ = println(identity(42));
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("42\n"));
    }

    #[test]
    fn expression_bodied_function_with_where_clause() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

protocol Doubler {
    func double() -> Self
}

extend std.num.Int64: Doubler {
    func double() -> std.num.Int64 = self + self
}

func doubleIt[T](x: T) -> T where T: Doubler = x.double()

func main() -> std.num.Int64 {
    let _ = println(doubleIt(21));
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("42\n"));
    }
}

mod protocols {
    use super::*;

    #[test]
    fn protocol_conformance_with_expression_body() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

protocol Valuable {
    func value() -> std.num.Int64
}

struct Thing: Valuable {
    let n: std.num.Int64

    func value() -> std.num.Int64 = self.n * 2
}

func main() -> std.num.Int64 {
    let t = Thing(n: 21);
    let _ = println(t.value());
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("42\n"));
    }
}

mod multiline {
    use super::*;

    #[test]
    fn expression_bodied_multiline_tuple() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func makePair(a: std.num.Int64, b: std.num.Int64) -> (std.num.Int64, std.num.Int64) =
    (
        a,
        b
    )

func main() -> std.num.Int64 {
    let (x, y) = makePair(3, 7);
    let _ = println(x);
    let _ = println(y);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("3\n7\n"));
    }

    #[test]
    fn expression_bodied_multiline_if() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func max(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 =
    if a > b { a }
    else { b }

func main() -> std.num.Int64 {
    let _ = println(max(10, 5));
    let _ = println(max(3, 8));
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("10\n8\n"));
    }
}

mod errors {
    use super::*;

    #[test]
    fn extern_function_cannot_have_expression_body() {
        Test::new(
            r#"
module Test

struct MyInt: Prelude.FFISafe { }

@extern(.C)
func external() -> MyInt = MyInt()
"#,
        )
        .with_stdlib()
        .expect(HasError("cannot have a body"));
    }
}
