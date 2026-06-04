// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // ASCII char = 1 byte
            let a: std.text.Char = 'a';
            if a.utf8Length() != 1 { return 1 }

            // Space = 1 byte
            let space: std.text.Char = ' ';
            if space.utf8Length() != 1 { return 2 }

            // DEL (0x7F) = 1 byte
            let del: std.text.Char = '\x7F';
            if del.utf8Length() != 1 { return 3 }

            0
        }
