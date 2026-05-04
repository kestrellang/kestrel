// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world hello";

            // ---- contains(matching:) ----
            let hasUpper = s.contains(matching: { (c) in c.isUppercase() });
            if hasUpper { return 1 }

            let hasLower = s.contains(matching: { (c) in c.isLowercase() });
            if hasLower == false { return 2 }

            // ---- firstIndex(matching:) ----
            let spacePos = s.firstIndex(matching: { (c) in c.isEqual(to: ' ') });
            if spacePos.isNone() { return 3 }
            if spacePos.unwrap() != 5 { return 4 }

            // firstIndex(matching:) no match
            let noMatch = s.firstIndex(matching: { (c) in c.isDigit() });
            if noMatch.isSome() { return 5 }

            // ---- lastIndex(of:) ----
            let lastHello = s.lastIndex(of: "hello");
            if lastHello.isNone() { return 6 }
            if lastHello.unwrap() != 12 { return 7 }

            // lastIndex(of:) first occurrence
            let firstWorld = s.lastIndex(of:"world");
            if firstWorld.isNone() { return 8 }
            if firstWorld.unwrap() != 6 { return 9 }

            // lastIndex(of:) no match
            let noRev = s.lastIndex(of:"xyz");
            if noRev.isSome() { return 10 }

            // lastIndex(of:) empty string
            let emptyRev = s.lastIndex(of:"");
            if emptyRev.isNone() { return 11 }
            // Should return length of string
            if emptyRev.unwrap() != 17 { return 12 }

            // ---- substringBytes(from:to:) ----
            let sub = s.substringBytes(from: 6, to: 11);
            if sub.isEqual(to: "world") == false { return 13 }

            // substringBytes with invalid range
            let badSub = s.substringBytes(from: 10, to: 5);
            if badSub.isEmpty == false { return 14 }

            // substringBytes from start
            let prefix = s.substringBytes(from: 0, to: 5);
            if prefix.isEqual(to: "hello") == false { return 15 }

            0
        }
