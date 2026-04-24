// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // byteSwapped — 2-byte swap
            // 0x0102 (258) byte-swapped = 0x0201 (513)
            let val: std.num.UInt16 = 258;
            let swapped = val.byteSwapped;
            let lit513: std.num.UInt16 = 513;
            if swapped.equals(lit513) == false { return 1 }

            // byteSwapped — 1 as u16 (0x0001) -> 0x0100 (256)
            let one: std.num.UInt16 = 1;
            let lit256: std.num.UInt16 = 256;
            if one.byteSwapped.equals(lit256) == false { return 2 }

            // leadingZeros — relative to 16-bit width
            if one.leadingZeros != 15 { return 3 }

            let zero: std.num.UInt16 = 0;
            if zero.leadingZeros != 16 { return 4 }

            let v32768: std.num.UInt16 = 32768;
            if v32768.leadingZeros != 0 { return 5 }

            // rotateLeft — 16-bit rotation
            // rotateLeft(1, by: 1) = 2
            let rotTwo: std.num.UInt16 = 2;
            if one.rotateLeft(by: 1).equals(rotTwo) == false { return 6 }
            // rotateLeft(32768, by: 1) = 1 (wraps around from bit 15 to bit 0)
            if v32768.rotateLeft(by: 1).equals(one) == false { return 7 }

            // rotateRight — 16-bit rotation
            // rotateRight(2, by: 1) = 1
            let two: std.num.UInt16 = 2;
            if two.rotateRight(by: 1).equals(one) == false { return 8 }
            // rotateRight(1, by: 1) = 32768 (wraps around from bit 0 to bit 15)
            if one.rotateRight(by: 1).equals(v32768) == false { return 9 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.num.UInt16 = 1234;
            if testVal.rotateLeft(by: 5).rotateRight(by: 5).equals(testVal) == false { return 10 }

            // init(from:) — from Int64
            let fromI64Val: std.num.Int64 = 50000;
            let fromI64 = std.num.UInt16(from: fromI64Val);
            let lit50000: std.num.UInt16 = 50000;
            if fromI64.equals(lit50000) == false { return 11 }

            // init(from:) — from UInt8
            let fromU8Val: std.num.UInt8 = 200;
            let fromU8 = std.num.UInt16(from: fromU8Val);
            let lit200: std.num.UInt16 = 200;
            if fromU8.equals(lit200) == false { return 12 }

            // parse — valid value
            let parsed = std.num.UInt16.parse( "65535");
            if parsed.isNone() { return 13 }
            if parsed.unwrap().equals(std.num.UInt16.maxValue) == false { return 14 }

            // parse — zero
            let parsedZero = std.num.UInt16.parse( "0");
            if parsedZero.isNone() { return 15 }
            if parsedZero.unwrap().equals(std.num.UInt16.zero) == false { return 16 }

            // parse — out of range (65536 > 65535)
            let parsedOver = std.num.UInt16.parse( "65536");
            if parsedOver.isSome() { return 17 }

            // parse — negative not allowed for unsigned
            let parsedNeg = std.num.UInt16.parse( "-1");
            if parsedNeg.isSome() { return 18 }

            // parse — empty string
            let parsedEmpty = std.num.UInt16.parse( "");
            if parsedEmpty.isSome() { return 19 }

            0
        }
