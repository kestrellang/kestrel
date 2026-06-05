// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // flatten() - nested arrays
            var inner1 = std.collections.Array[std.numeric.Int64]();
            inner1.append(1); inner1.append(2);
            var inner2 = std.collections.Array[std.numeric.Int64]();
            inner2.append(3); inner2.append(4);
            var inner3 = std.collections.Array[std.numeric.Int64]();
            inner3.append(5);
            var nested = std.collections.Array[std.collections.Array[std.numeric.Int64]]();
            nested.append(inner1); nested.append(inner2); nested.append(inner3);
            let flat = nested.flatten();
            if flat.count != 5 { return 1 }
            if flat(0) != 1 { return 2 }
            if flat(1) != 2 { return 3 }
            if flat(2) != 3 { return 4 }
            if flat(3) != 4 { return 5 }
            if flat(4) != 5 { return 6 }

            // flatten with empty inner arrays
            var mixedInner1 = std.collections.Array[std.numeric.Int64]();
            mixedInner1.append(1);
            let mixedInner2 = std.collections.Array[std.numeric.Int64]();
            var mixedInner3 = std.collections.Array[std.numeric.Int64]();
            mixedInner3.append(2); mixedInner3.append(3);
            var mixed = std.collections.Array[std.collections.Array[std.numeric.Int64]]();
            mixed.append(mixedInner1); mixed.append(mixedInner2); mixed.append(mixedInner3);
            let flatMixed = mixed.flatten();
            if flatMixed.count != 3 { return 7 }
            if flatMixed(0) != 1 { return 8 }
            if flatMixed(1) != 2 { return 9 }
            if flatMixed(2) != 3 { return 10 }

            // flatten empty outer array
            let emptyOuter = std.collections.Array[std.collections.Array[std.numeric.Int64]]();
            let flatEmpty = emptyOuter.flatten();
            if flatEmpty.count != 0 { return 11 }

            // joined(separator:) - positional single-name param
            var nums = std.collections.Array[std.numeric.Int64]();
            nums.append(1); nums.append(2); nums.append(3);
            let j = nums.joined(", ");
            if j != "1, 2, 3" { return 12 }

            // joined with empty separator (default)
            let j2 = nums.joined();
            if j2 != "123" { return 13 }

            // joined on empty array
            let emptyNums = std.collections.Array[std.numeric.Int64]();
            let jEmpty = emptyNums.joined(", ");
            if jEmpty != "" { return 14 }

            // joined single element
            var single = std.collections.Array[std.numeric.Int64]();
            single.append(42);
            let jSingle = single.joined("-");
            if jSingle != "42" { return 15 }

            0
        }
