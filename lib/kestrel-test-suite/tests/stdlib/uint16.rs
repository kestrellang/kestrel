use kestrel_test_suite::*;

#[test]
fn uint16_boundaries_and_constants() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // minValue is 0
            let minVal = std.num.UInt16.minValue;
            if minVal.equals(std.num.UInt16(intLiteral: 0)) == false { return 1 }

            // maxValue is 65535
            let maxVal = std.num.UInt16.maxValue;
            if maxVal.equals(std.num.UInt16(intLiteral: 65535)) == false { return 2 }

            // bitWidth is 16
            if std.num.UInt16.bitWidth != 16 { return 3 }

            // zero constant
            let z = std.num.UInt16.zero;
            if z.equals(std.num.UInt16(intLiteral: 0)) == false { return 4 }

            // one constant
            let o = std.num.UInt16.one;
            if o.equals(std.num.UInt16(intLiteral: 1)) == false { return 5 }

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
fn uint16_overflow_behavior() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let maxVal = std.num.UInt16.maxValue;
            let minVal = std.num.UInt16.minValue;
            let one = std.num.UInt16(intLiteral: 1);
            let two = std.num.UInt16(intLiteral: 2);
            let large = std.num.UInt16(intLiteral: 40000);

            // addChecked — normal case
            let addNorm = one.addChecked(two);
            if addNorm.isNone() { return 1 }
            if addNorm.unwrap().equals(std.num.UInt16(intLiteral: 3)) == false { return 2 }

            // addChecked — overflow at 65535
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
            let mulNorm = two.multiplyChecked(std.num.UInt16(intLiteral: 3));
            if mulNorm.isNone() { return 7 }
            if mulNorm.unwrap().equals(std.num.UInt16(intLiteral: 6)) == false { return 8 }

            // multiplyChecked — overflow near 65535
            let mulOver = large.multiplyChecked(two);
            if mulOver.isSome() { return 9 }

            // addSaturating — clamps to maxValue
            let addSat = maxVal.addSaturating(std.num.UInt16(intLiteral: 100));
            if addSat.equals(maxVal) == false { return 10 }

            // addSaturating — normal case
            let addSatNorm = one.addSaturating(two);
            if addSatNorm.equals(std.num.UInt16(intLiteral: 3)) == false { return 11 }

            // subtractSaturating — clamps to 0
            let subSat = minVal.subtractSaturating(one);
            if subSat.equals(std.num.UInt16.zero) == false { return 12 }

            // subtractSaturating — normal case
            let subSatNorm = two.subtractSaturating(one);
            if subSatNorm.equals(one) == false { return 13 }

            // multiplySaturating — clamps to maxValue
            let mulSat = large.multiplySaturating(two);
            if mulSat.equals(maxVal) == false { return 14 }

            // multiplySaturating — normal case
            let mulSatNorm = two.multiplySaturating(std.num.UInt16(intLiteral: 3));
            if mulSatNorm.equals(std.num.UInt16(intLiteral: 6)) == false { return 15 }

            // Subtraction wrapping behavior: 0 - 1 wraps to 65535
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
fn uint16_bitwidth_and_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // byteSwapped — 2-byte swap
            // 0x0102 (258) byte-swapped = 0x0201 (513)
            let val = std.num.UInt16(intLiteral: 258);
            let swapped = val.byteSwapped;
            if swapped.equals(std.num.UInt16(intLiteral: 513)) == false { return 1 }

            // byteSwapped — 1 as u16 (0x0001) -> 0x0100 (256)
            let one = std.num.UInt16(intLiteral: 1);
            if one.byteSwapped.equals(std.num.UInt16(intLiteral: 256)) == false { return 2 }

            // leadingZeros — relative to 16-bit width
            if one.leadingZeros != 15 { return 3 }

            let zero = std.num.UInt16(intLiteral: 0);
            if zero.leadingZeros != 16 { return 4 }

            let v32768 = std.num.UInt16(intLiteral: 32768);
            if v32768.leadingZeros != 0 { return 5 }

            // rotateLeft — 16-bit rotation
            // rotateLeft(1, by: 1) = 2
            if one.rotateLeft(by: 1).equals(std.num.UInt16(intLiteral: 2)) == false { return 6 }
            // rotateLeft(32768, by: 1) = 1 (wraps around from bit 15 to bit 0)
            if v32768.rotateLeft(by: 1).equals(one) == false { return 7 }

            // rotateRight — 16-bit rotation
            // rotateRight(2, by: 1) = 1
            let two = std.num.UInt16(intLiteral: 2);
            if two.rotateRight(by: 1).equals(one) == false { return 8 }
            // rotateRight(1, by: 1) = 32768 (wraps around from bit 0 to bit 15)
            if one.rotateRight(by: 1).equals(v32768) == false { return 9 }

            // rotateLeft and rotateRight are inverses
            let testVal = std.num.UInt16(intLiteral: 1234);
            if testVal.rotateLeft(by: 5).rotateRight(by: 5).equals(testVal) == false { return 10 }

            // init(from:) — from Int64
            let fromI64 = std.num.UInt16(from: std.num.Int64(intLiteral: 50000));
            if fromI64.equals(std.num.UInt16(intLiteral: 50000)) == false { return 11 }

            // init(from:) — from UInt8
            let fromU8 = std.num.UInt16(from: std.num.UInt8(intLiteral: 200));
            if fromU8.equals(std.num.UInt16(intLiteral: 200)) == false { return 12 }

            // parse — valid value
            let parsed = std.num.UInt16.parse(string: "65535");
            if parsed.isNone() { return 13 }
            if parsed.unwrap().equals(std.num.UInt16.maxValue) == false { return 14 }

            // parse — zero
            let parsedZero = std.num.UInt16.parse(string: "0");
            if parsedZero.isNone() { return 15 }
            if parsedZero.unwrap().equals(std.num.UInt16.zero) == false { return 16 }

            // parse — out of range (65536 > 65535)
            let parsedOver = std.num.UInt16.parse(string: "65536");
            if parsedOver.isSome() { return 17 }

            // parse — negative not allowed for unsigned
            let parsedNeg = std.num.UInt16.parse(string: "-1");
            if parsedNeg.isSome() { return 18 }

            // parse — empty string
            let parsedEmpty = std.num.UInt16.parse(string: "");
            if parsedEmpty.isSome() { return 19 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
