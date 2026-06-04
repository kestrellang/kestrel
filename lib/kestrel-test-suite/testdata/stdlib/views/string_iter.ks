// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let s: std.text.String = "hello";

            // ---- iter() on String ----
            let chars = s.iter().collect();
            if chars.count != 5 { return 1 }
            if chars(unchecked: 0).isEqual(to: 'h') == false { return 2 }
            if chars(unchecked: 4).isEqual(to: 'o') == false { return 3 }

            // iter() with map
            let upper = s.iter().map(as: { (c) in c.uppercased() }).collect();
            if upper.count != 5 { return 4 }
            if upper(unchecked: 0).isEqual(to: 'H') == false { return 5 }

            // iter() with filter
            let vowels = s.iter().filter(where: { (c) in
                c.isEqual(to: 'a') or c.isEqual(to: 'e') or c.isEqual(to: 'i') or c.isEqual(to: 'o') or c.isEqual(to: 'u')
            }).collect();
            if vowels.count != 2 { return 6 }

            // Empty string iter
            let empty = std.text.String();
            if empty.iter().count() != 0 { return 7 }

            0
        }
