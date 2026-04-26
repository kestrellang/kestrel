// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // ---- bytes(Range) - default subscript with range ----
            let sub = s.bytes(std.core.Range[std.num.Int64](0, 5));
            if sub.equals("hello") == false { return 1 }

            let sub2 = s.bytes(std.core.Range[std.num.Int64](6, 11));
            if sub2.equals("world") == false { return 2 }

            // ---- bytes(checked: Range) ----
            let checked = s.bytes(checked: std.core.Range[std.num.Int64](0, 5));
            if checked.isNone() { return 3 }
            if checked.unwrap().equals("hello") == false { return 4 }

            // Out of bounds returns None
            let oob = s.bytes(checked: std.core.Range[std.num.Int64](0, 100));
            if oob.isSome() { return 5 }

            // Negative start returns None
            let neg = s.bytes(checked: std.core.Range[std.num.Int64](-1, 5));
            if neg.isSome() { return 6 }

            // Start > end returns None
            let rev = s.bytes(checked: std.core.Range[std.num.Int64](5, 3));
            if rev.isSome() { return 7 }

            // Empty range
            let empty = s.bytes(checked: std.core.Range[std.num.Int64](3, 3));
            if empty.isNone() { return 8 }
            if empty.unwrap().isEmpty == false { return 9 }

            0
        }
