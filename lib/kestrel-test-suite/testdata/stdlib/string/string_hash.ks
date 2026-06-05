// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let a: std.text.String = "hello";
            let b: std.text.String = "world";

            // Hash different strings into separate hashers, verify they produce different values
            var hasher1 = std.collections.DefaultHasher();
            a.hash(into: hasher1);
            let hashA = hasher1.finish();

            var hasher2 = std.collections.DefaultHasher();
            b.hash(into: hasher2);
            let hashB = hasher2.finish();

            // Different strings should hash to different values
            if hashA == hashB { return 1 }

            // Same value should hash to same result (deterministic)
            var hasher3 = std.collections.DefaultHasher();
            a.hash(into: hasher3);
            let hashA2 = hasher3.finish();
            if hashA != hashA2 { return 2 }

            // Equal strings constructed independently should hash identically
            let a2: std.text.String = "hello";
            var hasher4 = std.collections.DefaultHasher();
            a2.hash(into: hasher4);
            let hashA3 = hasher4.finish();
            if hashA != hashA3 { return 3 }

            // Empty string should produce a valid hash
            let empty = std.text.String();
            var hasher5 = std.collections.DefaultHasher();
            empty.hash(into: hasher5);
            let hashEmpty = hasher5.finish();

            // Empty and non-empty should differ
            if hashEmpty == hashA { return 4 }

            // Strings that differ by one character should hash differently
            let c: std.text.String = "hellp";
            var hasher6 = std.collections.DefaultHasher();
            c.hash(into: hasher6);
            let hashC = hasher6.finish();
            if hashA == hashC { return 5 }

            0
        }
