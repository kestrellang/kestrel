use kestrel_test_suite::*;

#[test]
fn transitive_equality_constraints_in_extension_method() {
    Test::new(
        r#"module Test
        protocol Processor {
            type Input;
            type Output;
            func process(i: Input) -> Output
        }

        struct Pipeline[P] {
            var p: P
        }

        extend Pipeline[P] where P: Processor {
            func transform[T](t: T, i: P.Input) -> P.Output 
            where T: Processor, T.Input = P.Input, T.Output = P.Input {
                let intermediate = t.process(i);
                let twice = t.process(intermediate);
                return self.p.process(twice);
            }
        }
    "#,
    )
    .expect(Compiles);
}

#[test]
fn nested_associated_type_projections() {
    Test::new(
        r#"module Test
        protocol Level3 {
            static func baseValue() -> lang.i64
        }
        protocol Level2 {
            type Next: Level3;
            func level2() -> lang.i64
        }
        protocol Level1 {
            type Next: Level2;
        }

        struct S3: Level3 {
            static func baseValue() -> lang.i64 { return 300; }
        }
        struct S2: Level2 {
            type Next = S3;
            func level2() -> lang.i64 { return 2; }
        }
        struct S1: Level1 {
            type Next = S2;
        }

        struct Wrapper[T] { var val: T }

        extend Wrapper[T] where T: Level1 {
            func deepStatic() -> lang.i64 {
                return T.Next.Next.baseValue();
            }
        }
    "#,
    )
    .expect(Compiles);
}

#[test]
fn intentional_inference_failure_cases() {
    // Case 1: Wrong return type in extension method with constraint
    // CURRENT BUG: This succeeds when it should fail
    Test::new(
        r#"module Test
        protocol P { type A; func read() -> A }
        struct S[T] { var val: T }
        extend S[T] where T: P, T.A = lang.i64 {
            func fail_it() -> lang.str {
                return self.val.get(); 
            }
        }
    "#,
    )
    .expect(HasError("type mismatch"));
}
