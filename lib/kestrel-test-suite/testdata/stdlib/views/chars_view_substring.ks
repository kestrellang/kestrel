// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // ---- chars.substring(from:to:) ---- (character indices, not bytes)
            let sub = s.chars.substring(from: 0, to: 5);
            if sub.equals("hello") == false { return 1 }

            let sub2 = s.chars.substring(from: 6, to: 11);
            if sub2.equals("world") == false { return 2 }

            // ---- chars.substring(checked:to:) ----
            let checked = s.chars.substring(checked: 0, to: 5);
            if checked.isNone() { return 3 }
            if checked.unwrap().equals("hello") == false { return 4 }

            // Out of bounds returns None
            let oob = s.chars.substring(checked: 0, to: 100);
            if oob.isSome() { return 5 }

            // Empty range
            let empty = s.chars.substring(checked: 3, to: 3);
            if empty.isNone() { return 6 }
            if empty.unwrap().isEmpty == false { return 7 }

            0
        }
