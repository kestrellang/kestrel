// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test retain(where:)
            var s = std.collections.Set[std.numeric.Int64]();
             s.insert(1);
             s.insert(2);
             s.insert(3);
             s.insert(4);
             s.insert(5);

            s.retain(where: { (x) in x % 2 == 0 });
            if s.count != 2 { return 1 }
            if s.contains(2) == false { return 2 }
            if s.contains(4) == false { return 3 }
            if s.contains(1) { return 4 }
            if s.contains(3) { return 5 }

            // Test removeAll(where:)
            var s2 = std.collections.Set[std.numeric.Int64]();
             s2.insert(1);
             s2.insert(2);
             s2.insert(3);
             s2.insert(4);
             s2.insert(5);

            s2.removeAll(where: { (x) in x % 2 == 0 });
            if s2.count != 3 { return 6 }
            if s2.contains(1) == false { return 7 }
            if s2.contains(3) == false { return 8 }
            if s2.contains(5) == false { return 9 }
            if s2.contains(2) { return 10 }
            if s2.contains(4) { return 11 }

            0
        }
