// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- Grapheme from single char ----
            let g = std.text.Grapheme(char: 'a');
            if g.charCount() != 1 { return 1 }

            // ---- firstChar() ----
            let fc = g.firstChar();
            if fc.isNone() { return 2 }
            if fc.unwrap().equals('a') == false { return 3 }

            // ---- isAscii() ----
            if g.isAscii() == false { return 4 }

            // ---- utf8Length() ----
            if g.utf8Length() != 1 { return 5 }

            // ---- equals() ----
            let g2 = std.text.Grapheme(char: 'a');
            if g.equals(g2) == false { return 6 }

            let g3 = std.text.Grapheme(char: 'b');
            if g.equals(g3) { return 7 }

            // ---- Grapheme from multiple chars ----
            var chars = std.collections.Array[std.text.Char]();
            chars.append('a');
            chars.append('b');
            let gMulti = std.text.Grapheme(chars: chars);
            if gMulti.charCount() != 2 { return 8 }
            if gMulti.firstChar().unwrap().equals('a') == false { return 9 }
            if gMulti.utf8Length() != 2 { return 10 }

            // Multi-char grapheme is not ASCII
            if gMulti.isAscii() { return 11 }

            0
        }
