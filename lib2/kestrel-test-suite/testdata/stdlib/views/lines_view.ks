// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- lines.iter() with \n ----
            let s: std.text.String = "hello\nworld\nfoo";
            let lineArr = s.lines.iter().collect();
            if lineArr.count != 3 { return 1 }
            if lineArr(unchecked: 0).equals("hello") == false { return 2 }
            if lineArr(unchecked: 1).equals("world") == false { return 3 }
            if lineArr(unchecked: 2).equals("foo") == false { return 4 }

            // Single line (no newline)
            let single: std.text.String = "just one line";
            let singleLines = single.lines.iter().collect();
            if singleLines.count != 1 { return 5 }
            if singleLines(unchecked: 0).equals("just one line") == false { return 6 }

            // Trailing newline yields empty last line
            let trailing: std.text.String = "a\nb\n";
            let trailingLines = trailing.lines.iter().collect();
            if trailingLines.count != 2 { return 7 }
            if trailingLines(unchecked: 0).equals("a") == false { return 8 }
            if trailingLines(unchecked: 1).equals("b") == false { return 9 }

            // Empty string yields no lines
            let empty = std.text.String();
            let emptyLines = empty.lines.iter().collect();
            if emptyLines.count != 0 { return 10 }

            0
        }
