// test: execution
// stdlib: true

module Test

        @main
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
