// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // toBytesBigEndian / fromBytesBigEndian round-trip
            let val: std.numeric.Int64 = 258;  // 0x0000000000000102
            let bytes = val.toBytesBigEndian();
            if bytes.count != 8 { return 1 }
            // Big-endian: most significant byte first
            // 258 = 0x0000000000000102
            // bytes should be [0, 0, 0, 0, 0, 0, 1, 2]
            let b0 = std.numeric.Int64(from: bytes(0));
            let b1 = std.numeric.Int64(from: bytes(1));
            let b2 = std.numeric.Int64(from: bytes(2));
            let b3 = std.numeric.Int64(from: bytes(3));
            let b4 = std.numeric.Int64(from: bytes(4));
            let b5 = std.numeric.Int64(from: bytes(5));
            let b6 = std.numeric.Int64(from: bytes(6));
            let b7 = std.numeric.Int64(from: bytes(7));

            if b0 != 0 { return 2 }
            if b1 != 0 { return 3 }
            if b2 != 0 { return 4 }
            if b3 != 0 { return 5 }
            if b4 != 0 { return 6 }
            if b5 != 0 { return 7 }
            if b6 != 1 { return 8 }
            if b7 != 2 { return 9 }

            // fromBytesBigEndian round-trip
            let recovered = std.numeric.Int64(fromBytesBigEndian: bytes);
            if recovered.isNone() { return 10 }
            if recovered.unwrap() != 258 { return 11 }

            // Round-trip with zero
            let zeroVal: std.numeric.Int64 = 0;
            let zeroBytes = zeroVal.toBytesBigEndian();
            let zeroRecovered = std.numeric.Int64(fromBytesBigEndian: zeroBytes);
            if zeroRecovered.isNone() { return 12 }
            if zeroRecovered.unwrap() != 0 { return 13 }

            // Round-trip with negative value
            let negVal: std.numeric.Int64 = -1;
            let negBytes = negVal.toBytesBigEndian();
            let negRecovered = std.numeric.Int64(fromBytesBigEndian: negBytes);
            if negRecovered.isNone() { return 14 }
            if negRecovered.unwrap() != -1 { return 15 }

            0
        }
