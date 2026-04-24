// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // Test contains
            if s.contains("world") == false { return 1 }
            if s.contains("xyz") { return 2 }

            // Test find
            let pos = s.find("world");
            if pos.isNone() { return 3 }
            if pos.unwrap() != 6 { return 4 }

            // Test starts/ends with
            if s.starts(with: "hello") == false { return 5 }
            if s.starts(with: "world") { return 6 }

            // Test ends with
            if s.ends(with: "world") == false { return 7 }
            if s.ends(with: "hello") { return 8 }

            0
        }
