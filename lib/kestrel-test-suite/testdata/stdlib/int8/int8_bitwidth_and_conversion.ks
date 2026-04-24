// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // byteSwapped — identity for single-byte type
            let val: std.num.Int8 = 42;
            if val.byteSwapped != val { return 1 }
            let negVal: std.num.Int8 = -42;
            if negVal.byteSwapped != negVal { return 2 }

            // leadingZeros — relative to 8-bit width
            let one: std.num.Int8 = 1;
            if one.leadingZeros != 7 { return 3 }
            let zero: std.num.Int8 = 0;
            if zero.leadingZeros != 8 { return 4 }
            // -1 in Int8 is all 1s, so 0 leading zeros
            let negOne: std.num.Int8 = -1;
            if negOne.leadingZeros != 0 { return 5 }
            let four: std.num.Int8 = 4;
            // 4 = 0b00000100 -> 5 leading zeros
            if four.leadingZeros != 5 { return 6 }

            // trailingZeros — relative to 8-bit width
            if one.trailingZeros != 0 { return 7 }
            if zero.trailingZeros != 8 { return 8 }
            let eight: std.num.Int8 = 8;
            // 8 = 0b00001000 -> 3 trailing zeros
            if eight.trailingZeros != 3 { return 9 }

            // rotateLeft — 8-bit rotation
            // rotate 1 left by 1 = 2
            let expectedRotateTwo: std.num.Int8 = 2;
            if one.rotateLeft(by: 1) != expectedRotateTwo { return 10 }
            // rotate by 0 = unchanged
            if val.rotateLeft(by: 0) != val { return 11 }

            // rotateRight — 8-bit rotation
            // rotate 2 right by 1 = 1
            let two: std.num.Int8 = 2;
            if two.rotateRight(by: 1) != one { return 12 }
            // rotate by 0 = unchanged
            if val.rotateRight(by: 0) != val { return 13 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.num.Int8 = 37;
            if testVal.rotateLeft(by: 3).rotateRight(by: 3) != testVal { return 14 }

            // init(from:) — from Int64
            let i64val: std.num.Int64 = 100;
            let fromI64 = std.num.Int8(from: i64val);
            let expectedHundred: std.num.Int8 = 100;
            if fromI64 != expectedHundred { return 15 }

            // init(from:) — from Int64 negative
            let negI64: std.num.Int64 = -50;
            let fromNegI64 = std.num.Int8(from: negI64);
            let expectedNegFifty: std.num.Int8 = -50;
            if fromNegI64 != expectedNegFifty { return 16 }

            // parse — valid Int8 value
            let parsed = std.num.Int8.parse( "42");
            if parsed.isNone() { return 17 }
            if parsed.unwrap() != val { return 18 }

            // parse — negative value
            let parsedNeg = std.num.Int8.parse( "-128");
            if parsedNeg.isNone() { return 19 }
            if parsedNeg.unwrap() != std.num.Int8.minValue { return 20 }

            // parse — maxValue
            let parsedMax = std.num.Int8.parse( "127");
            if parsedMax.isNone() { return 21 }
            if parsedMax.unwrap() != std.num.Int8.maxValue { return 22 }

            // parse — out-of-range (too large)
            let parsedBig = std.num.Int8.parse( "128");
            if parsedBig.isSome() { return 23 }

            // parse — out-of-range (too small)
            let parsedSmall = std.num.Int8.parse( "-129");
            if parsedSmall.isSome() { return 24 }

            // parse — invalid string
            let parsedBad = std.num.Int8.parse( "abc");
            if parsedBad.isSome() { return 25 }

            // parse — empty string
            let parsedEmpty = std.num.Int8.parse( "");
            if parsedEmpty.isSome() { return 26 }

            0
        }
