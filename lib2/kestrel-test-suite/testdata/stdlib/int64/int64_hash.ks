// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let a: std.num.Int64 = 42;
            let b: std.num.Int64 = 43;

            // Hash different values into separate hashers, verify they produce different values
            var hasher1 = std.collections.DefaultHasher();
            a.hash(into: hasher1);
            let hashA = hasher1.finish();

            var hasher2 = std.collections.DefaultHasher();
            b.hash(into: hasher2);
            let hashB = hasher2.finish();

            // Different values should hash to different values
            if hashA == hashB { return 1 }

            // Same value should hash to same result (deterministic)
            var hasher3 = std.collections.DefaultHasher();
            a.hash(into: hasher3);
            let hashA2 = hasher3.finish();
            if hashA != hashA2 { return 2 }

            // Zero should produce a valid hash
            let zero: std.num.Int64 = 0;
            var hasher4 = std.collections.DefaultHasher();
            zero.hash(into: hasher4);
            let hashZero = hasher4.finish();

            // Zero and 42 should hash differently
            if hashZero == hashA { return 3 }

            // Negative value should produce a valid, distinct hash
            let neg: std.num.Int64 = -42;
            var hasher5 = std.collections.DefaultHasher();
            neg.hash(into: hasher5);
            let hashNeg = hasher5.finish();
            if hashNeg == hashA { return 4 }

            0
        }
