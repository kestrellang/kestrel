// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var s = std.collections.Set[std.numeric.Int64]();
            let _ = s.insert(1);

            // Insert contents of an array
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(1); // duplicate

            s.insert(contentsOf: arr);

            if s.count != 4 { return 1 }
            if s.contains(1) == false { return 2 }
            if s.contains(2) == false { return 3 }
            if s.contains(3) == false { return 4 }
            if s.contains(4) == false { return 5 }

            0
        }
