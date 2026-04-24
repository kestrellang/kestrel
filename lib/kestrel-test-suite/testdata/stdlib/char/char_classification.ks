// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- isAscii() ----
            let a: std.text.Char = 'a';
            if a.isAscii() == false { return 1 }

            // ---- isAlphabetic() ----
            if a.isAlphabetic() == false { return 2 }
            let z: std.text.Char = 'Z';
            if z.isAlphabetic() == false { return 3 }
            let digit: std.text.Char = '5';
            if digit.isAlphabetic() { return 4 }

            // ---- isDigit() ----
            if digit.isDigit() == false { return 5 }
            let zero: std.text.Char = '0';
            if zero.isDigit() == false { return 6 }
            let nine: std.text.Char = '9';
            if nine.isDigit() == false { return 7 }
            if a.isDigit() { return 8 }

            // ---- isAlphanumeric() ----
            if a.isAlphanumeric() == false { return 9 }
            if digit.isAlphanumeric() == false { return 10 }
            let space: std.text.Char = ' ';
            if space.isAlphanumeric() { return 11 }

            // ---- isWhitespace() ----
            if space.isWhitespace() == false { return 12 }
            let tab: std.text.Char = '\t';
            if tab.isWhitespace() == false { return 13 }
            let newline: std.text.Char = '\n';
            if newline.isWhitespace() == false { return 14 }
            if a.isWhitespace() { return 15 }

            // ---- isControl() ----
            let nul: std.text.Char = '\0';
            if nul.isControl() == false { return 16 }
            if newline.isControl() == false { return 17 }
            if a.isControl() { return 18 }

            // ---- isUppercase() / isLowercase() ----
            if a.isLowercase() == false { return 19 }
            if a.isUppercase() { return 20 }
            if z.isUppercase() == false { return 21 }
            if z.isLowercase() { return 22 }
            if digit.isUppercase() { return 23 }
            if digit.isLowercase() { return 24 }

            0
        }
