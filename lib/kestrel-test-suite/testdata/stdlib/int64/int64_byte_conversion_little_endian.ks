// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // toBytesLittleEndian / fromBytesLittleEndian round-trip
            let val: std.numeric.Int64 = 258;  // 0x0000000000000102
            let bytes = val.toBytesLittleEndian();
            if bytes.count != 8 { return 1 }
            // Little-endian: least significant byte first
            // 258 = 0x0000000000000102
            // bytes should be [2, 1, 0, 0, 0, 0, 0, 0]
            let b0 = std.numeric.Int64(from: bytes(0));
            let b1 = std.numeric.Int64(from: bytes(1));
            let b2 = std.numeric.Int64(from: bytes(2));
            let b7 = std.numeric.Int64(from: bytes(7));

            if b0 != 2 { return 2 }
            if b1 != 1 { return 3 }
            if b2 != 0 { return 4 }
            if b7 != 0 { return 5 }

            // fromBytesLittleEndian round-trip
            let recovered = std.numeric.Int64(fromBytesLittleEndian: bytes);
            if recovered.isNone() { return 6 }
            if recovered.unwrap() != 258 { return 7 }

            // Round-trip with a larger value
            let bigVal: std.numeric.Int64 = 1000000;
            let bigBytes = bigVal.toBytesLittleEndian();
            let bigRecovered = std.numeric.Int64(fromBytesLittleEndian: bigBytes);
            if bigRecovered.isNone() { return 8 }
            if bigRecovered.unwrap() != 1000000 { return 9 }

            // Round-trip with negative value
            let negVal: std.numeric.Int64 = -12345;
            let negBytes = negVal.toBytesLittleEndian();
            let negRecovered = std.numeric.Int64(fromBytesLittleEndian: negBytes);
            if negRecovered.isNone() { return 10 }
            if negRecovered.unwrap() != -12345 { return 11 }

            0
        }
