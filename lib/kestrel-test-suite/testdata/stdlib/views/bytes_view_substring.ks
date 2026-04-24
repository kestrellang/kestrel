// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // ---- bytes.substring(from:to:) ----
            let sub = s.bytes.substring(from: 0, to: 5);
            if sub.equals("hello") == false { return 1 }

            let sub2 = s.bytes.substring(from: 6, to: 11);
            if sub2.equals("world") == false { return 2 }

            // ---- bytes.substring(checked:to:) ----
            let checked = s.bytes.substring(checked: 0, to: 5);
            if checked.isNone() { return 3 }
            if checked.unwrap().equals("hello") == false { return 4 }

            // Out of bounds returns None
            let oob = s.bytes.substring(checked: 0, to: 100);
            if oob.isSome() { return 5 }

            // Negative start returns None
            let neg = s.bytes.substring(checked: -1, to: 5);
            if neg.isSome() { return 6 }

            // Start > end returns None
            let rev = s.bytes.substring(checked: 5, to: 3);
            if rev.isSome() { return 7 }

            // Empty range
            let empty = s.bytes.substring(checked: 3, to: 3);
            if empty.isNone() { return 8 }
            if empty.unwrap().isEmpty == false { return 9 }

            0
        }
