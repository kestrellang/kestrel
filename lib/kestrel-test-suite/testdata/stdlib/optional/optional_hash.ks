// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Hash test: Some(42) and Some(42) should hash the same
            // None and None should hash the same
            // Some(42) and None should hash differently (very likely)
            let a: std.result.Optional[std.num.Int64] = .Some(42);
            let b: std.result.Optional[std.num.Int64] = .Some(42);
            let c: std.result.Optional[std.num.Int64] = .None;

            var hasherA = std.collections.DefaultHasher();
            a.hash(into: hasherA);
            let hashA = hasherA.finish();

            var hasherB = std.collections.DefaultHasher();
            b.hash(into: hasherB);
            let hashB = hasherB.finish();

            var hasherC = std.collections.DefaultHasher();
            c.hash(into: hasherC);
            let hashC = hasherC.finish();

            // Equal values should produce equal hashes
            if hashA != hashB { return 1 }

            // Some and None should differ (in practice, always true)
            if hashA == hashC { return 2 }

            0
        }
