// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let s: std.text.String = "héllo\nworld";

            // BytesView.toString() round-trips the source byte-for-byte
            if s.bytes.toString().isEqual(to: "héllo\nworld") == false { return 1 }

            // CharsView.toString() yields the same bytes
            if s.chars.toString().isEqual(to: "héllo\nworld") == false { return 2 }

            // GraphemesView.toString() yields the same bytes
            if s.graphemes.toString().isEqual(to: "héllo\nworld") == false { return 3 }

            // LinesView.toString() preserves internal terminators
            if s.lines.toString().isEqual(to: "héllo\nworld") == false { return 4 }

            // CRLF / lone-CR preservation through LinesView.toString()
            let mixed: std.text.String = "a\r\nb\rc";
            if mixed.lines.toString().isEqual(to: "a\r\nb\rc") == false { return 5 }

            // toString on an empty view returns empty String
            let empty = std.text.String();
            if empty.bytes.toString().isEmpty == false { return 6 }
            if empty.chars.toString().isEmpty == false { return 7 }
            if empty.graphemes.toString().isEmpty == false { return 8 }
            if empty.lines.toString().isEmpty == false { return 9 }

            0
        }
