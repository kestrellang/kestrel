// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // ---- isAscii ----
            let a: std.text.Char = 'a';
            if a.isAscii == false { return 1 }

            // ---- isAsciiLetter ----
            if a.isAsciiLetter == false { return 2 }
            let z: std.text.Char = 'Z';
            if z.isAsciiLetter == false { return 3 }
            let digit: std.text.Char = '5';
            if digit.isAsciiLetter { return 4 }

            // ---- isAsciiDigit ----
            if digit.isAsciiDigit == false { return 5 }
            let zero: std.text.Char = '0';
            if zero.isAsciiDigit == false { return 6 }
            let nine: std.text.Char = '9';
            if nine.isAsciiDigit == false { return 7 }
            if a.isAsciiDigit { return 8 }

            // ---- isAsciiAlphanumeric ----
            if a.isAsciiAlphanumeric == false { return 9 }
            if digit.isAsciiAlphanumeric == false { return 10 }
            let space: std.text.Char = ' ';
            if space.isAsciiAlphanumeric { return 11 }

            // ---- isWhitespace ----
            if space.isWhitespace == false { return 12 }
            let tab: std.text.Char = '\t';
            if tab.isWhitespace == false { return 13 }
            let newline: std.text.Char = '\n';
            if newline.isWhitespace == false { return 14 }
            if a.isWhitespace { return 15 }

            // ---- isControl ----
            let nul: std.text.Char = '\0';
            if nul.isControl == false { return 16 }
            if newline.isControl == false { return 17 }
            if a.isControl { return 18 }

            // ---- isAsciiUppercase / isAsciiLowercase ----
            if a.isAsciiLowercase == false { return 19 }
            if a.isAsciiUppercase { return 20 }
            if z.isAsciiUppercase == false { return 21 }
            if z.isAsciiLowercase { return 22 }
            if digit.isAsciiUppercase { return 23 }
            if digit.isAsciiLowercase { return 24 }

            0
        }
