// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // byteSwapped — 4-byte swap
            // 0x01020304 (16909060) byte-swapped = 0x04030201 (67305985)
            let val: std.numeric.UInt32 = 16909060;
            let swapped = val.byteSwapped;
            let lit67305985: std.numeric.UInt32 = 67305985;
            if swapped.isEqual(to: lit67305985) == false { return 1 }

            // byteSwapped — 1 as u32 (0x00000001) -> 0x01000000 (16777216)
            let one: std.numeric.UInt32 = 1;
            let lit16777216: std.numeric.UInt32 = 16777216;
            if one.byteSwapped.isEqual(to: lit16777216) == false { return 2 }

            // leadingZeros — relative to 32-bit width
            if one.leadingZeros != 31 { return 3 }

            let zero: std.numeric.UInt32 = 0;
            if zero.leadingZeros != 32 { return 4 }

            let highBit: std.numeric.UInt32 = 2147483648;
            if highBit.leadingZeros != 0 { return 5 }

            // rotateLeft — 32-bit rotation
            // rotateLeft(1, by: 1) = 2
            let rotTwo: std.numeric.UInt32 = 2;
            if one.rotateLeft(by: 1).isEqual(to: rotTwo) == false { return 6 }
            // rotateLeft(2147483648, by: 1) = 1 (wraps around from bit 31 to bit 0)
            if highBit.rotateLeft(by: 1).isEqual(to: one) == false { return 7 }

            // rotateRight — 32-bit rotation
            // rotateRight(2, by: 1) = 1
            let two: std.numeric.UInt32 = 2;
            if two.rotateRight(by: 1).isEqual(to: one) == false { return 8 }
            // rotateRight(1, by: 1) = 2147483648 (wraps from bit 0 to bit 31)
            if one.rotateRight(by: 1).isEqual(to: highBit) == false { return 9 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.numeric.UInt32 = 123456;
            if testVal.rotateLeft(by: 11).rotateRight(by: 11).isEqual(to: testVal) == false { return 10 }

            // init(from:) — from Int64
            let fromI64Val: std.numeric.Int64 = 3000000000;
            let fromI64 = std.numeric.UInt32(from: fromI64Val);
            let lit3000000000: std.numeric.UInt32 = 3000000000;
            if fromI64.isEqual(to: lit3000000000) == false { return 11 }

            // init(from:) — from UInt16
            let fromU16Val: std.numeric.UInt16 = 50000;
            let fromU16 = std.numeric.UInt32(from: fromU16Val);
            let lit50000: std.numeric.UInt32 = 50000;
            if fromU16.isEqual(to: lit50000) == false { return 12 }

            // parse — valid value
            let parsed = std.numeric.UInt32(parsing: "4294967295");
            if parsed.isNone() { return 13 }
            if parsed.unwrap().isEqual(to: std.numeric.UInt32.maxValue) == false { return 14 }

            // parse — zero
            let parsedZero = std.numeric.UInt32(parsing: "0");
            if parsedZero.isNone() { return 15 }
            if parsedZero.unwrap().isEqual(to: std.numeric.UInt32.zero) == false { return 16 }

            // parse — out of range (4294967296 > 4294967295)
            let parsedOver = std.numeric.UInt32(parsing: "4294967296");
            if parsedOver.isSome() { return 17 }

            // parse — negative not allowed for unsigned
            let parsedNeg = std.numeric.UInt32(parsing: "-1");
            if parsedNeg.isSome() { return 18 }

            // parse — empty string
            let parsedEmpty = std.numeric.UInt32(parsing: "");
            if parsedEmpty.isSome() { return 19 }

            0
        }
