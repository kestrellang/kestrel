use kestrel_test_suite::*;

#[test]
fn reference_counting() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create RcBox
            let box1 = std.memory.RcBox[std.num.Int64](42);

            // Test getValue
            if box1.getValue() != 42 { return 1 }

            // Test initial refCount is 1
            if box1.refCount() != 1 { return 2 }

            // Test isUnique
            if box1.isUnique() == false { return 3 }

            // Test clone increments refCount
            let box2 = box1.clone();
            if box1.refCount() != 2 { return 4 }
            if box1.isUnique() { return 5 }

            // Both share the same value
            if box2.getValue() != 42 { return 6 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - deepClone() requires Cloneable on T, but Int64
// doesn't have a Cloneable witness. Monomorphization fails with
// "no witness found: protocol Cloneable for type Int64".
#[test]
fn set_value_and_deep_clone() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- setValue() ----
            let box1 = std.memory.RcBox[std.num.Int64](10);
            box1.setValue(42);
            if box1.getValue() != 42 { return 1 }

            // setValue on shared box affects both references
            let box2 = box1.clone();
            box1.setValue(99);
            if box2.getValue() != 99 { return 2 }

            // ---- deepClone() ----
            let box3 = std.memory.RcBox[std.num.Int64](50);
            let box4 = box3.deepClone();

            // Deep clone creates independent storage
            if box4.getValue() != 50 { return 3 }
            if box3.refCount() != 1 { return 4 }
            if box4.refCount() != 1 { return 5 }

            // Mutating deep clone doesn't affect original
            box4.setValue(100);
            if box3.getValue() != 50 { return 6 }
            if box4.getValue() != 100 { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
