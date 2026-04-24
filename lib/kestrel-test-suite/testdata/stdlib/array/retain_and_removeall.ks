// test: execution
// stdlib: true

module Test

        func makeArray5() -> std.collections.Array[std.num.Int64] {
            var a = std.collections.Array[std.num.Int64]();
            a.append(1); a.append(2); a.append(3); a.append(4); a.append(5);
            a
        }

        func main() -> lang.i64 {
            // Test retain(matching:)
            var arr = makeArray5();
            arr.retain(matching: { (x) in x % 2 == 0 });
            if arr.count != 2 { return 1 }
            if arr(0) != 2 { return 2 }
            if arr(1) != 4 { return 3 }

            // Test removeAll(matching:)
            var arr2 = makeArray5();
            arr2.removeAll(matching: { (x) in x % 2 == 0 });
            if arr2.count != 3 { return 4 }
            if arr2(0) != 1 { return 5 }
            if arr2(1) != 3 { return 6 }
            if arr2(2) != 5 { return 7 }

            // retain all - keeps everything
            var arr3 = std.collections.Array[std.num.Int64]();
            arr3.append(10); arr3.append(20); arr3.append(30);
            arr3.retain(matching: { (x) in true });
            if arr3.count != 3 { return 8 }

            // retain none - empties array
            var arr4 = std.collections.Array[std.num.Int64]();
            arr4.append(10); arr4.append(20); arr4.append(30);
            arr4.retain(matching: { (x) in false });
            if arr4.count != 0 { return 9 }

            // removeAll on empty array
            var arr5 = std.collections.Array[std.num.Int64]();
            arr5.removeAll(matching: { (x) in true });
            if arr5.count != 0 { return 10 }

            0
        }
