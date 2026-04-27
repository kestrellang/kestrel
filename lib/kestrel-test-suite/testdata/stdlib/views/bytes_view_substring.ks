// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // ---- bytes(Range) - default subscript with range yields BytesView ----
            let sub = s.bytes(std.core.Range[std.num.Int64](0, 5));
            if sub.count != std.num.Int64(intLiteral: 5) { return 1 }
            if sub.toString().equals("hello") == false { return 2 }

            let sub2 = s.bytes(std.core.Range[std.num.Int64](6, 11));
            if sub2.toString().equals("world") == false { return 3 }

            // ---- bytes.substring convenience (Range) ----
            if s.bytes.substring(std.core.Range[std.num.Int64](0, 5)).equals("hello") == false { return 4 }
            // ---- bytes.substring convenience (ClosedRange) ----
            if s.bytes.substring(std.core.ClosedRange[std.num.Int64](6, 10)).equals("world") == false { return 13 }

            // ---- bytes(checked: Range) ----
            let checked = s.bytes(checked: std.core.Range[std.num.Int64](0, 5));
            if checked.isNone() { return 5 }
            if checked.unwrap().toString().equals("hello") == false { return 6 }

            // Out of bounds returns None
            let oob = s.bytes(checked: std.core.Range[std.num.Int64](0, 100));
            if oob.isSome() { return 7 }

            // Negative start returns None
            let neg = s.bytes(checked: std.core.Range[std.num.Int64](-1, 5));
            if neg.isSome() { return 8 }

            // Start > end returns None
            let rev = s.bytes(checked: std.core.Range[std.num.Int64](5, 3));
            if rev.isSome() { return 9 }

            // Empty range
            let empty = s.bytes(checked: std.core.Range[std.num.Int64](3, 3));
            if empty.isNone() { return 10 }
            if empty.unwrap().count != std.num.Int64(intLiteral: 0) { return 11 }
            if empty.unwrap().toString().isEmpty == false { return 12 }

            0
        }
