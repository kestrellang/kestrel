use kestrel_test_suite::*;

#[test]
fn uint8_boundaries_and_constants() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // minValue is 0
            let minVal = std.num.UInt8.minValue;
            if minVal.equals(std.num.UInt8(intLiteral: 0)) == false { return 1 }

            // maxValue is 255
            let maxVal = std.num.UInt8.maxValue;
            if maxVal.equals(std.num.UInt8(intLiteral: 255)) == false { return 2 }

            // bitWidth is 8
            if std.num.UInt8.bitWidth != 8 { return 3 }

            // zero constant
            let z = std.num.UInt8.zero;
            if z.equals(std.num.UInt8(intLiteral: 0)) == false { return 4 }

            // one constant
            let o = std.num.UInt8.one;
            if o.equals(std.num.UInt8(intLiteral: 1)) == false { return 5 }

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
fn uint8_overflow_behavior() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let maxVal = std.num.UInt8.maxValue;
            let minVal = std.num.UInt8.minValue;
            let one = std.num.UInt8(intLiteral: 1);
            let two = std.num.UInt8(intLiteral: 2);
            let hundred = std.num.UInt8(intLiteral: 100);

            // addChecked — normal case
            let addNorm = one.addChecked(two);
            if addNorm.isNone() { return 1 }
            if addNorm.unwrap().equals(std.num.UInt8(intLiteral: 3)) == false { return 2 }

            // addChecked — overflow at 255
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
            let mulNorm = two.multiplyChecked(std.num.UInt8(intLiteral: 3));
            if mulNorm.isNone() { return 7 }
            if mulNorm.unwrap().equals(std.num.UInt8(intLiteral: 6)) == false { return 8 }

            // multiplyChecked — overflow near 255
            let mulOver = hundred.multiplyChecked(std.num.UInt8(intLiteral: 3));
            if mulOver.isSome() { return 9 }

            // addSaturating — clamps to maxValue
            let addSat = maxVal.addSaturating(std.num.UInt8(intLiteral: 10));
            if addSat.equals(maxVal) == false { return 10 }

            // addSaturating — normal case
            let addSatNorm = one.addSaturating(two);
            if addSatNorm.equals(std.num.UInt8(intLiteral: 3)) == false { return 11 }

            // subtractSaturating — clamps to 0 (no negative)
            let subSat = minVal.subtractSaturating(one);
            if subSat.equals(std.num.UInt8.zero) == false { return 12 }

            // subtractSaturating — normal case
            let subSatNorm = two.subtractSaturating(one);
            if subSatNorm.equals(one) == false { return 13 }

            // multiplySaturating — clamps to maxValue
            let mulSat = hundred.multiplySaturating(std.num.UInt8(intLiteral: 3));
            if mulSat.equals(maxVal) == false { return 14 }

            // multiplySaturating — normal case
            let mulSatNorm = two.multiplySaturating(std.num.UInt8(intLiteral: 3));
            if mulSatNorm.equals(std.num.UInt8(intLiteral: 6)) == false { return 15 }

            // Subtraction wrapping behavior: 0 - 1 wraps to 255
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
fn uint8_bitwidth_and_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // byteSwapped — identity for single-byte type
            let val = std.num.UInt8(intLiteral: 42);
            if val.byteSwapped.equals(val) == false { return 1 }
            let maxVal = std.num.UInt8.maxValue;
            if maxVal.byteSwapped.equals(maxVal) == false { return 2 }

            // leadingZeros — relative to 8-bit width
            let one = std.num.UInt8(intLiteral: 1);
            if one.leadingZeros != 7 { return 3 }

            let zero = std.num.UInt8(intLiteral: 0);
            if zero.leadingZeros != 8 { return 4 }

            let v128 = std.num.UInt8(intLiteral: 128);
            if v128.leadingZeros != 0 { return 5 }

            // rotateLeft — 8-bit rotation
            // rotateLeft(1, by: 1) = 2
            if one.rotateLeft(by: 1).equals(std.num.UInt8(intLiteral: 2)) == false { return 6 }
            // rotateLeft(128, by: 1) = 1 (wraps around)
            if v128.rotateLeft(by: 1).equals(one) == false { return 7 }

            // rotateRight — 8-bit rotation
            // rotateRight(2, by: 1) = 1
            let two = std.num.UInt8(intLiteral: 2);
            if two.rotateRight(by: 1).equals(one) == false { return 8 }
            // rotateRight(1, by: 1) = 128 (wraps around)
            if one.rotateRight(by: 1).equals(v128) == false { return 9 }

            // rotateLeft and rotateRight are inverses
            let testVal = std.num.UInt8(intLiteral: 42);
            if testVal.rotateLeft(by: 3).rotateRight(by: 3).equals(testVal) == false { return 10 }

            // init(from:) — from Int64
            let fromI64 = std.num.UInt8(from: std.num.Int64(intLiteral: 200));
            if fromI64.equals(std.num.UInt8(intLiteral: 200)) == false { return 11 }

            // init(from:) — from UInt64
            let fromU64 = std.num.UInt8(from: std.num.UInt64(intLiteral: 100));
            if fromU64.equals(std.num.UInt8(intLiteral: 100)) == false { return 12 }

            // parse — valid value
            let parsed = std.num.UInt8.parse(string: "255");
            if parsed.isNone() { return 13 }
            if parsed.unwrap().equals(std.num.UInt8.maxValue) == false { return 14 }

            // parse — zero
            let parsedZero = std.num.UInt8.parse(string: "0");
            if parsedZero.isNone() { return 15 }
            if parsedZero.unwrap().equals(std.num.UInt8.zero) == false { return 16 }

            // parse — out of range (256 > 255)
            let parsedOver = std.num.UInt8.parse(string: "256");
            if parsedOver.isSome() { return 17 }

            // parse — negative not allowed for unsigned
            let parsedNeg = std.num.UInt8.parse(string: "-1");
            if parsedNeg.isSome() { return 18 }

            // parse — empty string
            let parsedEmpty = std.num.UInt8.parse(string: "");
            if parsedEmpty.isSome() { return 19 }

            // parse — non-numeric
            let parsedBad = std.num.UInt8.parse(string: "abc");
            if parsedBad.isSome() { return 20 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
