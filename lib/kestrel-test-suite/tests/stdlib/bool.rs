use kestrel_test_suite::*;

#[test]
fn operations() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let t: std.core.Bool = true;
            let f: std.core.Bool = false;

            // Test and (uses closure-based logicalAnd)
            if (t and t) == false { return 1 }
            if t and f { return 2 }

            // Test or (uses closure-based logicalOr)
            if (t or f) == false { return 3 }
            if f or f { return 4 }

            // Test logicalNot
            if t.logicalNot() { return 5 }
            if f.logicalNot() == false { return 6 }

            // Test equals
            if t.equals(t) == false { return 7 }
            if t.equals(f) { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn bool_extended() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let t: std.core.Bool = true;
            let f: std.core.Bool = false;

            // Test matches
            if t.matches(t) == false { return 1 }
            if t.matches(f) { return 2 }
            if f.matches(f) == false { return 3 }
            if f.matches(t) { return 4 }

            // Test format - default formatting
            let trueStr = t.format();
            if trueStr != "true" { return 5 }

            let falseStr = f.format();
            if falseStr != "false" { return 6 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn bool_hash() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let t: std.core.Bool = true;
            let f: std.core.Bool = false;

            // Hash true and false into separate hashers, verify they produce different values
            var hasher1 = std.collections.DefaultHasher();
            t.hash(into: hasher1);
            let hashTrue = hasher1.finish();

            var hasher2 = std.collections.DefaultHasher();
            f.hash(into: hasher2);
            let hashFalse = hasher2.finish();

            // true and false should hash to different values
            if hashTrue == hashFalse { return 1 }

            // Same value should hash to same result (deterministic)
            var hasher3 = std.collections.DefaultHasher();
            t.hash(into: hasher3);
            let hashTrue2 = hasher3.finish();
            if hashTrue != hashTrue2 { return 2 }

            var hasher4 = std.collections.DefaultHasher();
            f.hash(into: hasher4);
            let hashFalse2 = hasher4.finish();
            if hashFalse != hashFalse2 { return 3 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
