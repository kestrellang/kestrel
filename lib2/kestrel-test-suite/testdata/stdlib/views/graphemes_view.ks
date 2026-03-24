// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello";

            // ---- graphemes.count() ----
            if s.graphemes.count() != 5 { return 1 }

            // ---- graphemes.iter() ----
            let gs = s.graphemes.iter().collect();
            if gs.count != 5 { return 2 }

            // Each grapheme is a single ASCII char
            let first = gs(unchecked: 0);
            if first.charCount() != 1 { return 3 }
            if first.firstChar().unwrap().equals('h') == false { return 4 }

            let last = gs(unchecked: 4);
            if last.firstChar().unwrap().equals('o') == false { return 5 }

            // Empty string
            let empty = std.text.String();
            if empty.graphemes.count() != 0 { return 6 }

            0
        }
