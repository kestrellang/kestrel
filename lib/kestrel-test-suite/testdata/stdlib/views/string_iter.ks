// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello";

            // ---- iter() on String ----
            let chars = s.iter().collect();
            if chars.count != 5 { return 1 }
            if chars(unchecked: 0).equals('h') == false { return 2 }
            if chars(unchecked: 4).equals('o') == false { return 3 }

            // iter() with map
            let upper = s.iter().map({ (c) in c.uppercased() }).collect();
            if upper.count != 5 { return 4 }
            if upper(unchecked: 0).equals('H') == false { return 5 }

            // iter() with filter
            let vowels = s.iter().filter(matching: { (c) in
                c.equals('a') or c.equals('e') or c.equals('i') or c.equals('o') or c.equals('u')
            }).collect();
            if vowels.count != 2 { return 6 }

            // Empty string iter
            let empty = std.text.String();
            if empty.iter().count() != 0 { return 7 }

            0
        }
