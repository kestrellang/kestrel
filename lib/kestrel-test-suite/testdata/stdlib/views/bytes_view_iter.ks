// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "abc";

            // ---- bytes.iter() ----
            let byteArr = s.bytes.iter().collect();
            if byteArr.count != 3 { return 1 }
            // 'a' = 97
            let byteA: std.numeric.UInt8 = 97;
            if byteArr(unchecked: 0) != byteA { return 2 }
            // 'b' = 98
            let byteB: std.numeric.UInt8 = 98;
            if byteArr(unchecked: 1) != byteB { return 3 }
            // 'c' = 99
            let byteC: std.numeric.UInt8 = 99;
            if byteArr(unchecked: 2) != byteC { return 4 }

            // Empty string yields empty iter
            let empty = std.text.String();
            let emptyBytes = empty.bytes.iter().collect();
            if emptyBytes.count != 0 { return 5 }

            0
        }
