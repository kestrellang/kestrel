// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // ---- chars(Range) - default subscript with range yields CharsView ----
            let sub = s.chars(std.core.Range[std.num.Int64](0, 5));
            if sub.toString().equals("hello") == false { return 1 }

            let sub2 = s.chars(std.core.Range[std.num.Int64](6, 11));
            if sub2.toString().equals("world") == false { return 2 }

            // ---- chars.substring convenience (Range) ----
            if s.chars.substring(std.core.Range[std.num.Int64](0, 5)).equals("hello") == false { return 3 }

            // ---- s.substring (defaults to chars, Range) ----
            if s.substring(std.core.Range[std.num.Int64](0, 5)).equals("hello") == false { return 4 }

            // ---- s.substring with ClosedRange ----
            if s.substring(std.core.ClosedRange[std.num.Int64](6, 10)).equals("world") == false { return 10 }

            // ---- chars(checked: Range) ----
            let checked = s.chars(checked: std.core.Range[std.num.Int64](0, 5));
            if checked.isNone() { return 5 }
            if checked.unwrap().toString().equals("hello") == false { return 6 }

            // Out of bounds returns None
            let oob = s.chars(checked: std.core.Range[std.num.Int64](0, 100));
            if oob.isSome() { return 7 }

            // Empty range
            let empty = s.chars(checked: std.core.Range[std.num.Int64](3, 3));
            if empty.isNone() { return 8 }
            if empty.unwrap().toString().isEmpty == false { return 9 }

            0
        }
