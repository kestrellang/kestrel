// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello";

            // ---- chars.count ----
            if s.chars().count != 5 { return 1 }

            // ---- chars.iter() ----
            let charArr = s.chars().iter().collect();
            if charArr.count != 5 { return 2 }
            if charArr(unchecked: 0).equals('h') == false { return 3 }
            if charArr(unchecked: 4).equals('o') == false { return 4 }

            // Empty string
            let empty = std.text.String();
            if empty.chars().count != 0 { return 5 }

            0
        }
