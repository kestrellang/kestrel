use kestrel_test_suite::*;

#[test]
fn int64_operations() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let a: std.num.Int64 = 10;
            let b: std.num.Int64 = 3;

            // Test arithmetic
            if a.add(b) != 13 { return 1 }
            if a.subtract(b) != 7 { return 2 }
            if a.multiply(b) != 30 { return 3 }
            if a.divide(b) != 3 { return 4 }
            if a.modulo(b) != 1 { return 5 }

            // Test negate and abs
            let neg: std.num.Int64 = -5;
            if neg.negate() != 5 { return 6 }
            if neg.abs() != 5 { return 7 }

            // Test comparison - a > b so compare should return Greater
            let cmp = a.compare(b);
            match cmp {
                .Greater => 0,
                _ => 8
            }
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_properties() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // sign
            let pos: std.num.Int64 = 42;
            let neg: std.num.Int64 = -42;
            let zero: std.num.Int64 = 0;

            if pos.sign != 1 { return 1 }
            if neg.sign != -1 { return 2 }
            if zero.sign != 0 { return 3 }

            // isPositive
            if pos.isPositive == false { return 4 }
            if zero.isPositive { return 5 }
            if neg.isPositive { return 6 }

            // isNegative
            if neg.isNegative == false { return 7 }
            if zero.isNegative { return 8 }
            if pos.isNegative { return 9 }

            // isZero
            if zero.isZero == false { return 10 }
            if pos.isZero { return 11 }

            // isPowerOfTwo
            let one: std.num.Int64 = 1;
            let two: std.num.Int64 = 2;
            let four: std.num.Int64 = 4;
            let three: std.num.Int64 = 3;
            if one.isPowerOfTwo == false { return 12 }
            if two.isPowerOfTwo == false { return 13 }
            if four.isPowerOfTwo == false { return 14 }
            if three.isPowerOfTwo { return 15 }
            if zero.isPowerOfTwo { return 16 }
            if neg.isPowerOfTwo { return 17 }

            // countOnes
            let ten: std.num.Int64 = 10;  // 0b1010 -> 2 ones
            if ten.countOnes != 2 { return 18 }
            if zero.countOnes != 0 { return 19 }
            let negOne: std.num.Int64 = -1;
            if negOne.countOnes != 64 { return 20 }

            // countZeros
            if zero.countZeros != 64 { return 21 }
            if negOne.countZeros != 0 { return 22 }

            // leadingZeros
            if one.leadingZeros != 63 { return 23 }
            if zero.leadingZeros != 64 { return 24 }
            if negOne.leadingZeros != 0 { return 25 }
            let val256: std.num.Int64 = 256;
            if val256.leadingZeros != 55 { return 26 }

            // trailingZeros
            let eight: std.num.Int64 = 8;
            if eight.trailingZeros != 3 { return 27 }
            let twelve: std.num.Int64 = 12;
            if twelve.trailingZeros != 2 { return 28 }
            if one.trailingZeros != 0 { return 29 }
            if zero.trailingZeros != 64 { return 30 }

            // byteSwapped
            let swapVal: std.num.Int64 = 1;
            // 1 as i64 in little-endian: 0x0000000000000001
            // byte-swapped: 0x0100000000000000 = 72057594037927936
            if swapVal.byteSwapped != 72057594037927936 { return 31 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_checked() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let a: std.num.Int64 = 10;
            let b: std.num.Int64 = 5;

            // addChecked - normal case
            let addResult = a.addChecked(b);
            if addResult.isNone() { return 1 }
            if addResult.unwrap() != 15 { return 2 }

            // addChecked - overflow
            let maxVal = std.num.Int64.maxValue;
            let overflowAdd = maxVal.addChecked(1);
            if overflowAdd.isSome() { return 3 }

            // subtractChecked - normal case
            let subResult = a.subtractChecked(b);
            if subResult.isNone() { return 4 }
            if subResult.unwrap() != 5 { return 5 }

            // subtractChecked - underflow
            let minVal = std.num.Int64.minValue;
            let overflowSub = minVal.subtractChecked(1);
            if overflowSub.isSome() { return 6 }

            // multiplyChecked - normal case
            let mulResult = a.multiplyChecked(b);
            if mulResult.isNone() { return 7 }
            if mulResult.unwrap() != 50 { return 8 }

            // multiplyChecked - overflow
            let overflowMul = maxVal.multiplyChecked(2);
            if overflowMul.isSome() { return 9 }

            // divideChecked - normal case
            let divResult = a.divideChecked(b);
            if divResult.isNone() { return 10 }
            if divResult.unwrap() != 2 { return 11 }

            // divideChecked - division by zero
            let zero: std.num.Int64 = 0;
            let divZero = a.divideChecked(zero);
            if divZero.isSome() { return 12 }

            // divideChecked - minValue / -1 overflow
            let negOne: std.num.Int64 = -1;
            let divOverflow = minVal.divideChecked(negOne);
            if divOverflow.isSome() { return 13 }

            // negateChecked - normal case
            let negResult = a.negateChecked();
            if negResult.isNone() { return 14 }
            if negResult.unwrap() != -10 { return 15 }

            // negateChecked - overflow (minValue)
            let negOverflow = minVal.negateChecked();
            if negOverflow.isSome() { return 16 }

            // absChecked - normal case
            let negFive: std.num.Int64 = -5;
            let absResult = negFive.absChecked();
            if absResult.isNone() { return 17 }
            if absResult.unwrap() != 5 { return 18 }

            // absChecked - overflow (minValue)
            let absOverflow = minVal.absChecked();
            if absOverflow.isSome() { return 19 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_saturating() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let a: std.num.Int64 = 10;
            let b: std.num.Int64 = 5;
            let maxVal = std.num.Int64.maxValue;
            let minVal = std.num.Int64.minValue;

            // addSaturating - normal case
            if a.addSaturating(b) != 15 { return 1 }

            // addSaturating - overflow clamps to maxValue
            if maxVal.addSaturating(1) != maxVal { return 2 }
            if maxVal.addSaturating(100) != maxVal { return 3 }

            // subtractSaturating - normal case
            if a.subtractSaturating(b) != 5 { return 4 }

            // subtractSaturating - underflow clamps to minValue
            if minVal.subtractSaturating(1) != minVal { return 5 }

            // multiplySaturating - normal case
            if a.multiplySaturating(b) != 50 { return 6 }

            // multiplySaturating - overflow clamps to maxValue
            if maxVal.multiplySaturating(2) != maxVal { return 7 }

            // multiplySaturating - negative overflow clamps to minValue
            let negTwo: std.num.Int64 = -2;
            if maxVal.multiplySaturating(negTwo) != minVal { return 8 }

            // negateSaturating - normal case
            let fortyTwo: std.num.Int64 = 42;
            if fortyTwo.negateSaturating() != -42 { return 9 }

            // negateSaturating - minValue clamps to maxValue
            if minVal.negateSaturating() != maxVal { return 10 }

            // absSaturating - normal case
            let negFortyTwo: std.num.Int64 = -42;
            if negFortyTwo.absSaturating() != 42 { return 11 }

            // absSaturating - minValue clamps to maxValue
            if minVal.absSaturating() != maxVal { return 12 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_extended() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // pow
            let two: std.num.Int64 = 2;
            if two.pow(10) != 1024 { return 1 }
            let three: std.num.Int64 = 3;
            if three.pow(4) != 81 { return 2 }
            let five: std.num.Int64 = 5;
            if five.pow(0) != 1 { return 3 }
            let negTwo: std.num.Int64 = -2;
            if negTwo.pow(3) != -8 { return 4 }

            // gcd
            let twelve: std.num.Int64 = 12;
            if twelve.gcd(8) != 4 { return 5 }
            let seventeen: std.num.Int64 = 17;
            if seventeen.gcd(13) != 1 { return 6 }
            let zero: std.num.Int64 = 0;
            if zero.gcd(5) != 5 { return 7 }
            let negTwelve: std.num.Int64 = -12;
            if negTwelve.gcd(8) != 4 { return 8 }

            // lcm
            let four: std.num.Int64 = 4;
            if four.lcm(6) != 12 { return 9 }
            if three.lcm(5) != 15 { return 10 }
            if zero.lcm(5) != 0 { return 11 }

            // clamp
            if five.clamp(0, 10) != 5 { return 12 }
            let negFive: std.num.Int64 = -5;
            if negFive.clamp(0, 10) != 0 { return 13 }
            let fifteen: std.num.Int64 = 15;
            if fifteen.clamp(0, 10) != 10 { return 14 }

            // successor
            if five.successor() != 6 { return 15 }
            let negOne: std.num.Int64 = -1;
            if negOne.successor() != 0 { return 16 }

            // predecessor
            if five.predecessor() != 4 { return 17 }
            if zero.predecessor() != -1 { return 18 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_bitwise() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // bitwiseAnd: 0b1100 & 0b1010 = 0b1000 = 8
            let a: std.num.Int64 = 12;
            let b: std.num.Int64 = 10;
            if a.bitwiseAnd(b) != 8 { return 1 }

            // bitwiseOr: 0b1100 | 0b1010 = 0b1110 = 14
            if a.bitwiseOr(b) != 14 { return 2 }

            // bitwiseXor: 0b1100 ^ 0b1010 = 0b0110 = 6
            if a.bitwiseXor(b) != 6 { return 3 }

            // bitwiseNot: ~0 = -1
            let zero: std.num.Int64 = 0;
            if zero.bitwiseNot() != -1 { return 4 }
            // bitwiseNot: ~(-1) = 0
            let negOne: std.num.Int64 = -1;
            if negOne.bitwiseNot() != 0 { return 5 }

            // shiftLeft: 1 << 4 = 16
            let one: std.num.Int64 = 1;
            if one.shiftLeft(by: 4) != 16 { return 6 }

            // shiftRight: 16 >> 2 = 4
            let sixteen: std.num.Int64 = 16;
            if sixteen.shiftRight(by: 2) != 4 { return 7 }

            // shiftRight with negative preserves sign: -16 >> 2 = -4
            let negSixteen: std.num.Int64 = -16;
            if negSixteen.shiftRight(by: 2) != -4 { return 8 }

            // rotateLeft
            // rotate 1 left by 1: should be 2
            if one.rotateLeft(by: 1) != 2 { return 9 }
            // rotate by 0: unchanged
            let val: std.num.Int64 = 42;
            if val.rotateLeft(by: 0) != 42 { return 10 }

            // rotateRight
            // rotate 2 right by 1: should be 1
            let two: std.num.Int64 = 2;
            if two.rotateRight(by: 1) != 1 { return 11 }
            // rotate by 0: unchanged
            if val.rotateRight(by: 0) != 42 { return 12 }

            // Verify rotateLeft and rotateRight are inverses
            let testVal: std.num.Int64 = 12345;
            if testVal.rotateLeft(by: 7).rotateRight(by: 7) != 12345 { return 13 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_parsing() {
    // TODO: Overloaded static method Int64.parse cannot be resolved by the compiler.
    // The compiler reports "could not infer type" for Int64.parse calls because
    // the two overloads (1-arg and 2-arg) prevent type inference from succeeding.
    // This test should be updated to .expect(Compiles).expect(Runs) once
    // overloaded static method resolution is fixed.
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let p1 = std.num.Int64.parse("42", 10);
            if p1.isNone() { return 1 }
            if p1.unwrap() != 42 { return 2 }

            let p7 = std.num.Int64.parse("ff", 16);
            if p7.isNone() { return 3 }
            if p7.unwrap() != 255 { return 4 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(HasError("could not infer type"));
}

#[test]
fn int64_compound_assignment() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // addAssign
            var x: std.num.Int64 = 10;
            x.addAssign(5);
            if x != 15 { return 1 }

            // subtractAssign
            var y: std.num.Int64 = 20;
            y.subtractAssign(8);
            if y != 12 { return 2 }

            // multiplyAssign
            var z: std.num.Int64 = 6;
            z.multiplyAssign(7);
            if z != 42 { return 3 }

            // divideAssign
            var d: std.num.Int64 = 100;
            d.divideAssign(4);
            if d != 25 { return 4 }

            // modAssign
            var m: std.num.Int64 = 17;
            m.modAssign(5);
            if m != 2 { return 5 }

            // bitwiseAndAssign: 0b1111 & 0b1010 = 0b1010 = 10
            var ba: std.num.Int64 = 15;
            ba.bitwiseAndAssign(10);
            if ba != 10 { return 6 }

            // bitwiseOrAssign: 0b1100 | 0b0011 = 0b1111 = 15
            var bo: std.num.Int64 = 12;
            bo.bitwiseOrAssign(3);
            if bo != 15 { return 7 }

            // bitwiseXorAssign: 0b1111 ^ 0b1010 = 0b0101 = 5
            var bx: std.num.Int64 = 15;
            bx.bitwiseXorAssign(10);
            if bx != 5 { return 8 }

            // shiftLeftAssign: 1 << 4 = 16
            var sl: std.num.Int64 = 1;
            sl.shiftLeftAssign(by: 4);
            if sl != 16 { return 9 }

            // shiftRightAssign: 32 >> 2 = 8
            var sr: std.num.Int64 = 32;
            sr.shiftRightAssign(by: 2);
            if sr != 8 { return 10 }

            // Chained compound assignments
            var v: std.num.Int64 = 5;
            v.addAssign(3);       // 8
            v.multiplyAssign(2);  // 16
            v.subtractAssign(6);  // 10
            v.divideAssign(2);    // 5
            if v != 5 { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_byte_conversion_big_endian() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // toBytesBigEndian / fromBytesBigEndian round-trip
            let val: std.num.Int64 = 258;  // 0x0000000000000102
            let bytes = val.toBytesBigEndian();
            if bytes.count != 8 { return 1 }
            // Big-endian: most significant byte first
            // 258 = 0x0000000000000102
            // bytes should be [0, 0, 0, 0, 0, 0, 1, 2]
            let b0 = std.num.Int64(from: bytes(0));
            let b1 = std.num.Int64(from: bytes(1));
            let b2 = std.num.Int64(from: bytes(2));
            let b3 = std.num.Int64(from: bytes(3));
            let b4 = std.num.Int64(from: bytes(4));
            let b5 = std.num.Int64(from: bytes(5));
            let b6 = std.num.Int64(from: bytes(6));
            let b7 = std.num.Int64(from: bytes(7));

            if b0 != 0 { return 2 }
            if b1 != 0 { return 3 }
            if b2 != 0 { return 4 }
            if b3 != 0 { return 5 }
            if b4 != 0 { return 6 }
            if b5 != 0 { return 7 }
            if b6 != 1 { return 8 }
            if b7 != 2 { return 9 }

            // fromBytesBigEndian round-trip
            let recovered = std.num.Int64.fromBytesBigEndian(bytes);
            if recovered.isNone() { return 10 }
            if recovered.unwrap() != 258 { return 11 }

            // Round-trip with zero
            let zeroVal: std.num.Int64 = 0;
            let zeroBytes = zeroVal.toBytesBigEndian();
            let zeroRecovered = std.num.Int64.fromBytesBigEndian(zeroBytes);
            if zeroRecovered.isNone() { return 12 }
            if zeroRecovered.unwrap() != 0 { return 13 }

            // Round-trip with negative value
            let negVal: std.num.Int64 = -1;
            let negBytes = negVal.toBytesBigEndian();
            let negRecovered = std.num.Int64.fromBytesBigEndian(negBytes);
            if negRecovered.isNone() { return 14 }
            if negRecovered.unwrap() != -1 { return 15 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_byte_conversion_little_endian() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // toBytesLittleEndian / fromBytesLittleEndian round-trip
            let val: std.num.Int64 = 258;  // 0x0000000000000102
            let bytes = val.toBytesLittleEndian();
            if bytes.count != 8 { return 1 }
            // Little-endian: least significant byte first
            // 258 = 0x0000000000000102
            // bytes should be [2, 1, 0, 0, 0, 0, 0, 0]
            let b0 = std.num.Int64(from: bytes(0));
            let b1 = std.num.Int64(from: bytes(1));
            let b2 = std.num.Int64(from: bytes(2));
            let b7 = std.num.Int64(from: bytes(7));

            if b0 != 2 { return 2 }
            if b1 != 1 { return 3 }
            if b2 != 0 { return 4 }
            if b7 != 0 { return 5 }

            // fromBytesLittleEndian round-trip
            let recovered = std.num.Int64.fromBytesLittleEndian(bytes);
            if recovered.isNone() { return 6 }
            if recovered.unwrap() != 258 { return 7 }

            // Round-trip with a larger value
            let bigVal: std.num.Int64 = 1000000;
            let bigBytes = bigVal.toBytesLittleEndian();
            let bigRecovered = std.num.Int64.fromBytesLittleEndian(bigBytes);
            if bigRecovered.isNone() { return 8 }
            if bigRecovered.unwrap() != 1000000 { return 9 }

            // Round-trip with negative value
            let negVal: std.num.Int64 = -12345;
            let negBytes = negVal.toBytesLittleEndian();
            let negRecovered = std.num.Int64.fromBytesLittleEndian(negBytes);
            if negRecovered.isNone() { return 10 }
            if negRecovered.unwrap() != -12345 { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_byte_conversion_native() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // toBytes / fromBytes round-trip (native byte order)
            let val: std.num.Int64 = 42;
            let bytes = val.toBytes();
            if bytes.count != 8 { return 1 }

            let recovered = std.num.Int64.fromBytes(bytes);
            if recovered.isNone() { return 2 }
            if recovered.unwrap() != 42 { return 3 }

            // Round-trip with negative value
            let negVal: std.num.Int64 = -999;
            let negBytes = negVal.toBytes();
            let negRecovered = std.num.Int64.fromBytes(negBytes);
            if negRecovered.isNone() { return 4 }
            if negRecovered.unwrap() != -999 { return 5 }

            // fromBytes with wrong number of bytes returns None
            var shortBytes = std.collections.Array[std.num.UInt8]();
            shortBytes.append(std.num.UInt8(intLiteral: 1));
            shortBytes.append(std.num.UInt8(intLiteral: 2));
            shortBytes.append(std.num.UInt8(intLiteral: 3));
            let shortResult = std.num.Int64.fromBytes(shortBytes);
            if shortResult.isSome() { return 6 }

            // fromBytes with empty array returns None
            let emptyBytes = std.collections.Array[std.num.UInt8]();
            let emptyResult = std.num.Int64.fromBytes(emptyBytes);
            if emptyResult.isSome() { return 7 }

            // Round-trip with zero
            let zeroVal: std.num.Int64 = 0;
            let zeroBytes = zeroVal.toBytes();
            let zeroRecovered = std.num.Int64.fromBytes(zeroBytes);
            if zeroRecovered.isNone() { return 8 }
            if zeroRecovered.unwrap() != 0 { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_equals_method() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let a: std.num.Int64 = 42;
            let b: std.num.Int64 = 42;
            let c: std.num.Int64 = 43;

            // equals with same value
            if a.equals(b) == false { return 1 }

            // equals with different value
            if a.equals(c) { return 2 }

            // equals with zero
            let zero: std.num.Int64 = 0;
            if zero.equals(0) == false { return 3 }
            if zero.equals(1) { return 4 }

            // equals with negative values
            let neg: std.num.Int64 = -10;
            if neg.equals(-10) == false { return 5 }
            if neg.equals(10) { return 6 }

            // equals is symmetric
            if a.equals(b) != b.equals(a) { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_hash() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let a: std.num.Int64 = 42;
            let b: std.num.Int64 = 43;

            // Hash different values into separate hashers, verify they produce different values
            var hasher1 = std.collections.DefaultHasher();
            a.hash(into: hasher1);
            let hashA = hasher1.finish();

            var hasher2 = std.collections.DefaultHasher();
            b.hash(into: hasher2);
            let hashB = hasher2.finish();

            // Different values should hash to different values
            if hashA == hashB { return 1 }

            // Same value should hash to same result (deterministic)
            var hasher3 = std.collections.DefaultHasher();
            a.hash(into: hasher3);
            let hashA2 = hasher3.finish();
            if hashA != hashA2 { return 2 }

            // Zero should produce a valid hash
            let zero: std.num.Int64 = 0;
            var hasher4 = std.collections.DefaultHasher();
            zero.hash(into: hasher4);
            let hashZero = hasher4.finish();

            // Zero and 42 should hash differently
            if hashZero == hashA { return 3 }

            // Negative value should produce a valid, distinct hash
            let neg: std.num.Int64 = -42;
            var hasher5 = std.collections.DefaultHasher();
            neg.hash(into: hasher5);
            let hashNeg = hasher5.finish();
            if hashNeg == hashA { return 4 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int64_format_options() {
    Test::new(
        r#"module Test

        func makeOpts(radix: std.num.Int64, uppercase: std.core.Bool, alternate: std.core.Bool) -> std.text.FormatOptions {
            var opts = std.text.FormatOptions();
            opts.radix = radix;
            opts.uppercase = uppercase;
            opts.alternate = alternate;
            opts
        }

        func makeWidthOpts(width: std.num.Int64, alignment: std.text.Alignment, fill: std.text.Char) -> std.text.FormatOptions {
            var opts = std.text.FormatOptions();
            opts.width = .Some(width);
            opts.alignment = alignment;
            opts.fill = fill;
            opts
        }

        func makeSignOpts(sign: std.text.Sign) -> std.text.FormatOptions {
            var opts = std.text.FormatOptions();
            opts.sign = sign;
            opts
        }

        func main() -> lang.i64 {
            let val: std.num.Int64 = 255;

            // Default format (decimal)
            let dec = val.format();
            if dec.equals("255") == false { return 1 }

            // Hexadecimal lowercase
            let hex = val.format(makeOpts(16, false, false));
            if hex.equals("ff") == false { return 2 }

            // Hexadecimal uppercase
            let hexUp = val.format(makeOpts(16, true, false));
            if hexUp.equals("FF") == false { return 3 }

            // Hex with alternate form (0x prefix)
            let hexAlt = val.format(makeOpts(16, false, true));
            if hexAlt.equals("0xff") == false { return 4 }

            // Binary
            let fortyTwo: std.num.Int64 = 42;
            let bin = fortyTwo.format(makeOpts(2, false, false));
            if bin.equals("101010") == false { return 5 }

            // Binary with alternate form (0b prefix)
            let binAlt = fortyTwo.format(makeOpts(2, false, true));
            if binAlt.equals("0b101010") == false { return 6 }

            // Octal
            let oct = val.format(makeOpts(8, false, false));
            if oct.equals("377") == false { return 7 }

            // Octal with alternate form (0o prefix)
            let octAlt = val.format(makeOpts(8, false, true));
            if octAlt.equals("0o377") == false { return 8 }

            // Zero formats as "0"
            let zero: std.num.Int64 = 0;
            let zeroStr = zero.format();
            if zeroStr.equals("0") == false { return 9 }

            // Negative value
            let neg: std.num.Int64 = -42;
            let negStr = neg.format();
            if negStr.equals("-42") == false { return 10 }

            // Width and right alignment (default for numbers)
            let padded = fortyTwo.format(makeWidthOpts(6, std.text.Alignment.Right, ' '));
            if padded.equals("    42") == false { return 11 }

            // Width with zero-fill and right alignment
            let zeroPad = fortyTwo.format(makeWidthOpts(6, std.text.Alignment.Right, '0'));
            if zeroPad.equals("000042") == false { return 12 }

            // Width with left alignment
            let leftPad = fortyTwo.format(makeWidthOpts(6, std.text.Alignment.Left, ' '));
            if leftPad.equals("42    ") == false { return 13 }

            // Sign always
            let signAlways = fortyTwo.format(makeSignOpts(std.text.Sign.Always));
            if signAlways.equals("+42") == false { return 14 }

            // Sign space
            let signSpace = fortyTwo.format(makeSignOpts(std.text.Sign.Space));
            if signSpace.equals(" 42") == false { return 15 }

            // Negative with sign always still shows minus
            let negAlways = neg.format(makeSignOpts(std.text.Sign.Always));
            if negAlways.equals("-42") == false { return 16 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
