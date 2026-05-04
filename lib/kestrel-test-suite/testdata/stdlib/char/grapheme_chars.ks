// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Single-char grapheme: chars() returns array with one element
            let g1 = std.text.Grapheme(char: 'x');
            let c1 = g1.chars();
            if c1.count != 1 { return 1 }
            if c1(unchecked: 0).isEqual(to: 'x') == false { return 2 }

            // Multi-char grapheme: chars() returns the array of chars
            var arr = std.collections.Array[std.text.Char]();
            arr.append('a');
            arr.append('b');
            arr.append('c');
            let g2 = std.text.Grapheme(chars: arr);
            let c2 = g2.chars();
            if c2.count != 3 { return 3 }
            if c2(unchecked: 0).isEqual(to: 'a') == false { return 4 }
            if c2(unchecked: 1).isEqual(to: 'b') == false { return 5 }
            if c2(unchecked: 2).isEqual(to: 'c') == false { return 6 }

            0
        }
