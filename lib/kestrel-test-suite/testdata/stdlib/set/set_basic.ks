// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test init() and isEmpty
            var s = std.collections.Set[std.numeric.Int64]();
            if s.isEmpty == false { return 1 }
            if s.count != 0 { return 2 }

            // Test insert() - returns true for new element
            let inserted1 = s.insert(10);
            if inserted1 == false { return 3 }
            if s.count != 1 { return 4 }

            // Test insert() - returns false for existing element
            let inserted2 = s.insert(10);
            if inserted2 { return 5 }
            if s.count != 1 { return 6 }

            // Test contains()
            s.insert(20);
            s.insert(30);
            if s.contains(10) == false { return 7 }
            if s.contains(20) == false { return 8 }
            if s.contains(999) { return 9 }

            // Test isEmpty after inserts
            if s.isEmpty { return 10 }

            // Test remove() - returns true for existing element
            let removed1 = s.remove(20);
            if removed1 == false { return 11 }
            if s.count != 2 { return 12 }
            if s.contains(20) { return 13 }

            // Test remove() - returns false for missing element
            let removed2 = s.remove(999);
            if removed2 { return 14 }

            // Test clear()
            s.clear();
            if s.count != 0 { return 15 }
            if s.isEmpty == false { return 16 }
            if s.contains(10) { return 17 }

            0
        }
