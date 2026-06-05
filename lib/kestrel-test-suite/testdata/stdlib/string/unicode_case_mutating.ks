// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // lowercase() mutating
            var s1: std.text.String = "HELLO WORLD";
            s1.lowercase();
            if s1.isEqual(to: "hello world") == false { return 1 }

            // uppercase() mutating
            var s2: std.text.String = "hello world";
            s2.uppercase();
            if s2.isEqual(to: "HELLO WORLD") == false { return 2 }

            // lowercase on already lowercase
            var s3: std.text.String = "already lower";
            s3.lowercase();
            if s3.isEqual(to: "already lower") == false { return 3 }

            // uppercase on already uppercase
            var s4: std.text.String = "ALREADY UPPER";
            s4.uppercase();
            if s4.isEqual(to: "ALREADY UPPER") == false { return 4 }

            // lowercase on mixed case
            var s5: std.text.String = "HeLLo WoRLd";
            s5.lowercase();
            if s5.isEqual(to: "hello world") == false { return 5 }

            // uppercase on mixed case
            var s6: std.text.String = "HeLLo WoRLd";
            s6.uppercase();
            if s6.isEqual(to: "HELLO WORLD") == false { return 6 }

            // lowercase on empty string
            var s7 = std.text.String();
            s7.lowercase();
            if s7.isEmpty == false { return 7 }

            // uppercase on empty string
            var s8 = std.text.String();
            s8.uppercase();
            if s8.isEmpty == false { return 8 }

            // lowercase preserves non-alpha chars
            var s9: std.text.String = "Hello 123!";
            s9.lowercase();
            if s9.isEqual(to: "hello 123!") == false { return 9 }

            // uppercase preserves non-alpha chars
            var s10: std.text.String = "Hello 123!";
            s10.uppercase();
            if s10.isEqual(to: "HELLO 123!") == false { return 10 }

            0
        }
