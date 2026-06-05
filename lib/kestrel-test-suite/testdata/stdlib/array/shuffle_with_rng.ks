// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(4); arr.append(5);

            // shuffle(using:) with a seeded RNG for deterministic results
            var rng = std.numeric.Lcg64(seed: 42);
            arr.shuffle(using: rng);

            // After shuffle, same count and same elements
            if arr.count != 5 { return 1 }
            if arr.contains(1) == false { return 2 }
            if arr.contains(2) == false { return 3 }
            if arr.contains(3) == false { return 4 }
            if arr.contains(4) == false { return 5 }
            if arr.contains(5) == false { return 6 }

            // shuffled(using:) returns new array, original unchanged
            var arr2 = std.collections.Array[std.numeric.Int64]();
            arr2.append(10); arr2.append(20); arr2.append(30);
            var rng2 = std.numeric.Lcg64(seed: 123);
            let result = arr2.shuffled(using: rng2);
            if result.count != 3 { return 7 }
            if result.contains(10) == false { return 8 }
            if result.contains(20) == false { return 9 }
            if result.contains(30) == false { return 10 }

            // Original unchanged
            if arr2(0) != 10 { return 11 }
            if arr2(1) != 20 { return 12 }
            if arr2(2) != 30 { return 13 }

            // Deterministic: same seed gives same result
            var arr3 = std.collections.Array[std.numeric.Int64]();
            arr3.append(1); arr3.append(2); arr3.append(3); arr3.append(4); arr3.append(5);
            var rng3a = std.numeric.Lcg64(seed: 999);
            arr3.shuffle(using: rng3a);

            var arr4 = std.collections.Array[std.numeric.Int64]();
            arr4.append(1); arr4.append(2); arr4.append(3); arr4.append(4); arr4.append(5);
            var rng3b = std.numeric.Lcg64(seed: 999);
            arr4.shuffle(using: rng3b);

            if arr3.isEqual(to: arr4) == false { return 14 }

            0
        }
