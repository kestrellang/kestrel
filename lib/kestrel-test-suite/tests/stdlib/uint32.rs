use kestrel_test_suite::*;

#[test]
fn uint32_boundaries_and_constants() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // minValue is 0
            let minVal = std.num.UInt32.minValue;
            if minVal.equals(std.num.UInt32(intLiteral: 0)) == false { return 1 }

            // maxValue is 4294967295
            let maxVal = std.num.UInt32.maxValue;
            if maxVal.equals(std.num.UInt32(intLiteral: 4294967295)) == false { return 2 }

            // bitWidth is 32
            if std.num.UInt32.bitWidth != 32 { return 3 }

            // zero constant
            let z = std.num.UInt32.zero;
            if z.equals(std.num.UInt32(intLiteral: 0)) == false { return 4 }

            // one constant
            let o = std.num.UInt32.one;
            if o.equals(std.num.UInt32(intLiteral: 1)) == false { return 5 }

            // isZero
            if minVal.isZero == false { return 6 }
            if maxVal.isZero { return 7 }

            // isPositive
            if maxVal.isPositive == false { return 8 }
            if minVal.isPositive { return 9 }

            // isNegative is always false for unsigned
            if minVal.isNegative { return 10 }
            if maxVal.isNegative { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn uint32_overflow_behavior() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let maxVal = std.num.UInt32.maxValue;
            let minVal = std.num.UInt32.minValue;
            let one = std.num.UInt32(intLiteral: 1);
            let two = std.num.UInt32(intLiteral: 2);
            let large = std.num.UInt32(intLiteral: 3000000000);

            // addChecked — normal case
            let addNorm = one.addChecked(two);
            if addNorm.isNone() { return 1 }
            if addNorm.unwrap().equals(std.num.UInt32(intLiteral: 3)) == false { return 2 }

            // addChecked — overflow at 4294967295
            let addOver = maxVal.addChecked(one);
            if addOver.isSome() { return 3 }

            // subtractChecked — normal case
            let subNorm = two.subtractChecked(one);
            if subNorm.isNone() { return 4 }
            if subNorm.unwrap().equals(one) == false { return 5 }

            // subtractChecked — underflow at 0
            let subUnder = minVal.subtractChecked(one);
            if subUnder.isSome() { return 6 }

            // multiplyChecked — normal case
            let mulNorm = two.multiplyChecked(std.num.UInt32(intLiteral: 3));
            if mulNorm.isNone() { return 7 }
            if mulNorm.unwrap().equals(std.num.UInt32(intLiteral: 6)) == false { return 8 }

            // multiplyChecked — overflow near 4294967295
            let mulOver = large.multiplyChecked(two);
            if mulOver.isSome() { return 9 }

            // addSaturating — clamps to maxValue
            let addSat = maxVal.addSaturating(std.num.UInt32(intLiteral: 100));
            if addSat.equals(maxVal) == false { return 10 }

            // addSaturating — normal case
            let addSatNorm = one.addSaturating(two);
            if addSatNorm.equals(std.num.UInt32(intLiteral: 3)) == false { return 11 }

            // subtractSaturating — clamps to 0
            let subSat = minVal.subtractSaturating(one);
            if subSat.equals(std.num.UInt32.zero) == false { return 12 }

            // subtractSaturating — normal case
            let subSatNorm = two.subtractSaturating(one);
            if subSatNorm.equals(one) == false { return 13 }

            // multiplySaturating — clamps to maxValue
            let mulSat = large.multiplySaturating(two);
            if mulSat.equals(maxVal) == false { return 14 }

            // multiplySaturating — normal case
            let mulSatNorm = two.multiplySaturating(std.num.UInt32(intLiteral: 3));
            if mulSatNorm.equals(std.num.UInt32(intLiteral: 6)) == false { return 15 }

            // Subtraction wrapping behavior: 0 - 1 wraps to 4294967295
            let wrapped = minVal.subtract(one);
            if wrapped.equals(maxVal) == false { return 16 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- likely parse() or init(from:) issue with type resolution
#[test]
fn uint32_bitwidth_and_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // byteSwapped — 4-byte swap
            // 0x01020304 (16909060) byte-swapped = 0x04030201 (67305985)
            let val = std.num.UInt32(intLiteral: 16909060);
            let swapped = val.byteSwapped;
            if swapped.equals(std.num.UInt32(intLiteral: 67305985)) == false { return 1 }

            // byteSwapped — 1 as u32 (0x00000001) -> 0x01000000 (16777216)
            let one = std.num.UInt32(intLiteral: 1);
            if one.byteSwapped.equals(std.num.UInt32(intLiteral: 16777216)) == false { return 2 }

            // leadingZeros — relative to 32-bit width
            if one.leadingZeros != 31 { return 3 }

            let zero = std.num.UInt32(intLiteral: 0);
            if zero.leadingZeros != 32 { return 4 }

            let highBit = std.num.UInt32(intLiteral: 2147483648);
            if highBit.leadingZeros != 0 { return 5 }

            // rotateLeft — 32-bit rotation
            // rotateLeft(1, by: 1) = 2
            if one.rotateLeft(by: 1).equals(std.num.UInt32(intLiteral: 2)) == false { return 6 }
            // rotateLeft(2147483648, by: 1) = 1 (wraps around from bit 31 to bit 0)
            if highBit.rotateLeft(by: 1).equals(one) == false { return 7 }

            // rotateRight — 32-bit rotation
            // rotateRight(2, by: 1) = 1
            let two = std.num.UInt32(intLiteral: 2);
            if two.rotateRight(by: 1).equals(one) == false { return 8 }
            // rotateRight(1, by: 1) = 2147483648 (wraps from bit 0 to bit 31)
            if one.rotateRight(by: 1).equals(highBit) == false { return 9 }

            // rotateLeft and rotateRight are inverses
            let testVal = std.num.UInt32(intLiteral: 123456);
            if testVal.rotateLeft(by: 11).rotateRight(by: 11).equals(testVal) == false { return 10 }

            // init(from:) — from Int64
            let fromI64 = std.num.UInt32(from: std.num.Int64(intLiteral: 3000000000));
            if fromI64.equals(std.num.UInt32(intLiteral: 3000000000)) == false { return 11 }

            // init(from:) — from UInt16
            let fromU16 = std.num.UInt32(from: std.num.UInt16(intLiteral: 50000));
            if fromU16.equals(std.num.UInt32(intLiteral: 50000)) == false { return 12 }

            // parse — valid value
            let parsed = std.num.UInt32.parse(string: "4294967295");
            if parsed.isNone() { return 13 }
            if parsed.unwrap().equals(std.num.UInt32.maxValue) == false { return 14 }

            // parse — zero
            let parsedZero = std.num.UInt32.parse(string: "0");
            if parsedZero.isNone() { return 15 }
            if parsedZero.unwrap().equals(std.num.UInt32.zero) == false { return 16 }

            // parse — out of range (4294967296 > 4294967295)
            let parsedOver = std.num.UInt32.parse(string: "4294967296");
            if parsedOver.isSome() { return 17 }

            // parse — negative not allowed for unsigned
            let parsedNeg = std.num.UInt32.parse(string: "-1");
            if parsedNeg.isSome() { return 18 }

            // parse — empty string
            let parsedEmpty = std.num.UInt32.parse(string: "");
            if parsedEmpty.isSome() { return 19 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
