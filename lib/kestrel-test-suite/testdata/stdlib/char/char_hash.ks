// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let a: std.text.Char = 'a';
            let b: std.text.Char = 'b';

            // Hash 'a' and 'b' into separate hashers, verify they produce different values
            var hasher1 = std.collections.DefaultHasher();
            a.hash(into: hasher1);
            let hashA = hasher1.finish();

            var hasher2 = std.collections.DefaultHasher();
            b.hash(into: hasher2);
            let hashB = hasher2.finish();

            // Different chars should hash to different values
            if hashA == hashB { return 1 }

            // Same value should hash to same result (deterministic)
            var hasher3 = std.collections.DefaultHasher();
            a.hash(into: hasher3);
            let hashA2 = hasher3.finish();
            if hashA != hashA2 { return 2 }

            // Equal chars constructed differently should hash identically
            let a2 = std.text.Char(std.num.UInt32(intLiteral: 97));  // 'a' = 97
            var hasher4 = std.collections.DefaultHasher();
            a2.hash(into: hasher4);
            let hashA3 = hasher4.finish();
            if hashA != hashA3 { return 3 }

            0
        }
