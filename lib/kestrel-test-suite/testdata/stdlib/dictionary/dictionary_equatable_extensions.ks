// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // equals(other:) - same dictionaries
            var a = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = a.insert(1, 10);
            let _ = a.insert(2, 20);
            let _ = a.insert(3, 30);

            var b = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = b.insert(1, 10);
            let _ = b.insert(2, 20);
            let _ = b.insert(3, 30);

            if a.equals(b) == false { return 1 }

            // equals - different values
            var c = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = c.insert(1, 10);
            let _ = c.insert(2, 99);
            let _ = c.insert(3, 30);
            if a.equals(c) { return 2 }

            // equals - different count
            var d = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = d.insert(1, 10);
            let _ = d.insert(2, 20);
            if a.equals(d) { return 3 }

            // equals - empty dictionaries
            let e1 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let e2 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            if e1.equals(e2) == false { return 4 }

            // containsValue(value:) - value exists
            if a.containsValue(20) == false { return 5 }

            // containsValue(value:) - value not found
            if a.containsValue(999) { return 6 }

            // firstKey(of:) - value exists
            let fk = a.firstKey(of: 20);
            if fk.isNone() { return 7 }
            if fk.unwrap() != 2 { return 8 }

            // firstKey(of:) - value not found
            let fkNone = a.firstKey(of: 999);
            if fkNone.isSome() { return 9 }

            // allKeys(of:) - single match
            let keys10 = a.allKeys(of: 10);
            if keys10.count != 1 { return 10 }
            if keys10(0) != 1 { return 11 }

            // allKeys(of:) - multiple matches
            var multi = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = multi.insert(1, 100);
            let _ = multi.insert(2, 200);
            let _ = multi.insert(3, 100);
            let _ = multi.insert(4, 300);
            let _ = multi.insert(5, 100);
            let keys100 = multi.allKeys(of: 100);
            if keys100.count != 3 { return 12 }
            // All keys with value 100 should be present (1, 3, 5)
            if keys100.contains(1) == false { return 13 }
            if keys100.contains(3) == false { return 14 }
            if keys100.contains(5) == false { return 15 }

            // allKeys(of:) - no match
            let keysNone = a.allKeys(of: 999);
            if keysNone.count != 0 { return 16 }

            0
        }
