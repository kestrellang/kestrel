//! Pattern matching MIR tests.
//!
//! Tests for pattern matching including:
//! - Match expressions
//! - If-let expressions
//! - Guard-let statements
//! - While-let loops
//! - Pattern bindings

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

// ============================================================================
// MATCH EXPRESSIONS
// ============================================================================

mod match_expressions {
    use super::*;

    #[test]
    fn match_on_int() {
        Test::new(
            r#"
            module Main

            func describe(n: lang.i64) -> lang.i64 {
                match n {
                    0 => 0,
                    1 => 1,
                    _ => 2
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.describe$n")
                .returns(MirTy::I64)
                .has_at_least_blocks(3),
        );
    }

    #[test]
    fn match_on_bool() {
        Test::new(
            r#"
            module Main

            func toInt(b: lang.i1) -> lang.i64 {
                match b {
                    true => 1,
                    false => 0
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.toInt$b")
                .returns(MirTy::I64)
                .any_block(|b| b.terminates_with(TerminatorPattern::Branch)),
        );
    }

    #[test]
    fn match_with_binding() {
        Test::new(
            r#"
            module Main

            enum Option {
                case Some(value: lang.i64)
                case None
            }

            func unwrap(opt: Option) -> lang.i64 {
                match opt {
                    .Some(v) => v,
                    .None => 0
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.unwrap$opt")
                .returns(MirTy::I64)
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }

    #[test]
    fn match_with_wildcard() {
        Test::new(
            r#"
            module Main

            enum Color {
                case Red
                case Green
                case Blue
            }

            func isRed(c: Color) -> lang.i1 {
                match c {
                    .Red => true,
                    _ => false
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.isRed$c")
                .returns(MirTy::Bool)
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }
}

// ============================================================================
// IF-LET EXPRESSIONS
// ============================================================================

mod if_let {
    use super::*;

    #[test]
    fn simple_if_let() {
        // Based on tmp/14_if_let.ks
        Test::new(
            r#"
            module Main

            enum Option[T] {
                case Some(value: T)
                case None
            }

            func unwrap(opt: Option[lang.i64]) -> lang.i64 {
                if let .Some(v) = opt {
                    v
                } else {
                    0
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.unwrap$opt")
                .returns(MirTy::I64)
                .has_at_least_blocks(3), // entry, then, else
        );
    }

    #[test]
    fn if_let_chain() {
        // Based on tmp/14_if_let.ks
        Test::new(
            r#"
            module Main

            enum Option[T] {
                case Some(value: T)
                case None
            }

            func addIfBothSome(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
                if let .Some(x) = a, let .Some(y) = b {
                    lang.i64_add(x, y)
                } else {
                    0
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.addIfBothSome$a$b")
                .returns(MirTy::I64)
                .has_at_least_blocks(4), // entry, first match, second match, else
        );
    }

    #[test]
    fn if_let_without_else() {
        Test::new(
            r#"
            module Main

            enum Option[T] {
                case Some(value: T)
                case None
            }

            func maybeDouble(opt: Option[lang.i64]) -> lang.i64 {
                var result: lang.i64 = 0;
                if let .Some(v) = opt {
                    result = lang.i64_mul(v, 2);
                }
                result
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.maybeDouble$opt")
                .returns(MirTy::I64)
                .has_local("result", MirTy::I64),
        );
    }

    #[test]
    fn if_let_optional_type_operator() {
        Test::new(
            r#"
            module Main

            func unwrap(opt: std.num.Int64?) -> std.num.Int64 {
                if let .Some(v) = opt {
                    v
                } else {
                    0
                }
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.unwrap$opt")
                .has_at_least_blocks(3)
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }
}

// ============================================================================
// GUARD-LET STATEMENTS
// ============================================================================

mod guard_let {
    use super::*;

    #[test]
    fn simple_guard_let() {
        // Based on tmp/15_guard_let.ks
        Test::new(
            r#"
            module Main

            enum Option[T] {
                case Some(value: T)
                case None
            }

            func process(opt: Option[lang.i64]) -> lang.i64 {
                guard let .Some(v) = opt else {
                    return 0
                }
                lang.i64_mul(v, 2)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.process$opt")
                .returns(MirTy::I64)
                .has_at_least_blocks(3), // entry, guard body, continuation
        );
    }

    #[test]
    fn multiple_guard_lets() {
        Test::new(
            r#"
            module Main

            enum Option[T] {
                case Some(value: T)
                case None
            }

            func process(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
                guard let .Some(x) = a else {
                    return 0
                }
                guard let .Some(y) = b else {
                    return 0
                }
                lang.i64_add(x, y)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.process$a$b")
                .returns(MirTy::I64)
                .has_at_least_blocks(4),
        );
    }
}

// ============================================================================
// WHILE-LET LOOPS
// ============================================================================

mod while_let {
    use super::*;

    #[test]
    fn simple_while_let() {
        // Based on tmp/18_while_let.ks
        Test::new(
            r#"
            module Main

            enum Option[T] {
                case Some(value: T)
                case None
            }

            struct Counter {
                var count: lang.i64

                init(start: lang.i64) {
                    self.count = start;
                }

                mutating func next() -> Option[lang.i64] {
                    if lang.i64_signed_gt(self.count, 0) {
                        let v = self.count;
                        self.count = lang.i64_sub(self.count, 1);
                        Option[lang.i64].Some(value: v)
                    } else {
                        Option[lang.i64].None
                    }
                }
            }

            func sumAll() -> lang.i64 {
                var counter = Counter(5);
                var sum: lang.i64 = 0;
                while let .Some(v) = counter.next() {
                    sum = lang.i64_add(sum, v);
                }
                sum
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.sumAll")
                .returns(MirTy::I64)
                .has_local("counter", MirTy::named("Main.Counter"))
                .has_local("sum", MirTy::I64)
                .calls("Main.Counter.next"),
        );
    }
}

// ============================================================================
// TUPLE PATTERNS
// ============================================================================

mod tuple_patterns {
    use super::*;

    #[test]
    fn match_on_tuple() {
        Test::new(
            r#"
            module Main

            func classify(pair: (lang.i64, lang.i64)) -> lang.i64 {
                match pair {
                    (0, 0) => 0,
                    (0, _) => 1,
                    (_, 0) => 2,
                    _ => 3
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.classify$pair")
                .returns(MirTy::I64)
                .has_at_least_blocks(4),
        );
    }

    #[test]
    fn destructure_tuple() {
        Test::new(
            r#"
            module Main

            func sum(pair: (lang.i64, lang.i64)) -> lang.i64 {
                let (a, b) = pair;
                lang.i64_add(a, b)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.sum$pair")
                .returns(MirTy::I64)
                .has_local("a", MirTy::I64)
                .has_local("b", MirTy::I64),
        );
    }
}

// NOTE: Struct patterns (like `Point(x: 0, y: 0)`) are not yet supported in the parser

// ============================================================================
// NESTED PATTERNS
// ============================================================================

mod nested_patterns {
    use super::*;

    #[test]
    fn nested_enum_pattern() {
        Test::new(
            r#"
            module Main

            enum Inner {
                case A(x: lang.i64)
                case B
            }

            enum Outer {
                case Wrap(inner: Inner)
                case Empty
            }

            func extract(o: Outer) -> lang.i64 {
                match o {
                    .Wrap(.A(x)) => x,
                    .Wrap(.B) => 0,
                    .Empty => 0
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.extract$o")
                .returns(MirTy::I64)
                .has_at_least_blocks(4),
        );
    }
}

// ============================================================================
// MATCH IN CLOSURES
// ============================================================================

mod match_in_closures {
    use super::*;

    #[test]
    fn closure_with_match() {
        // Based on tmp/21_closure_in_match.ks
        Test::new(
            r#"
            module Main

            enum Option {
                case Some(value: lang.i64)
                case None
            }

            func main() -> lang.i64 {
                let unwrap: (Option) -> lang.i64 = { (opt: Option) in
                    match opt {
                        .Some(v) => v,
                        .None => 0
                    }
                };
                unwrap(Option.Some(value: 42))
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.main")
                .returns(MirTy::I64)
                .calls_escaping(),
        )
        .expect(
            Mir::mir_closure("Main.main", 0)
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }
}

// ============================================================================
// MATCH WITH GUARDS
// ============================================================================

mod match_with_guards {
    use super::*;

    #[test]
    fn match_with_if_guard() {
        // Based on tmp/30_match_with_guards.ks
        Test::new(
            r#"
            module Main

            func classify(n: lang.i64) -> lang.i64 {
                match n {
                    x if lang.i64_signed_lt(x, 0) => lang.i64_sub(0, 1),
                    x if lang.i64_eq(x, 0) => 0,
                    x if lang.i64_signed_lt(x, 10) => 1,
                    _ => 2
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.classify$n")
                .returns(MirTy::I64)
                .has_at_least_blocks(5), // Multiple guards create multiple branches
        );
    }
}
