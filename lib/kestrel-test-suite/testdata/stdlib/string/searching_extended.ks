// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world hello";

            // ---- contains(matching:) ----
            let hasUpper = s.contains(matching: { (c) in c.isAsciiUppercase() });
            if hasUpper { return 1 }

            let hasLower = s.contains(matching: { (c) in c.isAsciiLowercase() });
            if hasLower == false { return 2 }

            // ---- chars.firstIndex(matching:) ----
            let spacePos = s.chars.firstIndex(matching: { (c) in c.isEqual(to: ' ') });
            if spacePos.isNone() { return 3 }
            if spacePos.unwrap().byteOffset != 5 { return 4 }

            // chars.firstIndex(matching:) no match
            let noMatch = s.chars.firstIndex(matching: { (c) in c.isAsciiDigit() });
            if noMatch.isSome() { return 5 }

            // ---- lastIndex(of:) via Str ----
            let lastHello = s.lastIndex(of: "hello");
            if lastHello.isNone() { return 6 }
            if lastHello.unwrap().value != 12 { return 7 }

            // lastIndex(of:) first occurrence
            let firstWorld = s.lastIndex(of:"world");
            if firstWorld.isNone() { return 8 }
            if firstWorld.unwrap().value != 6 { return 9 }

            // lastIndex(of:) no match
            let noRev = s.lastIndex(of:"xyz");
            if noRev.isSome() { return 10 }

            // lastIndex(of:) empty string
            let emptyRev = s.lastIndex(of:"");
            if emptyRev.isNone() { return 11 }
            // Should return length of string
            if emptyRev.unwrap().value != 17 { return 12 }

            // ---- slice subslice ----
            let sub = s.asSlice().subslice(from: 6, to: 11);
            if sub.toOwned().isEqual(to: "world") == false { return 13 }

            // subslice with invalid range (start > end)
            let badSub = s.asSlice().subslice(from: 10, to: 5);
            if badSub.isEmpty == false { return 14 }

            // subslice from start
            let prefix = s.asSlice().subslice(from: 0, to: 5);
            if prefix.toOwned().isEqual(to: "hello") == false { return 15 }

            0
        }
