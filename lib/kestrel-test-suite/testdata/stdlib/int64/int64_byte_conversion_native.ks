// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // toBytes / fromBytes round-trip (native byte order)
            let val: std.numeric.Int64 = 42;
            let bytes = val.toBytes();
            if bytes.count != 8 { return 1 }

            let recovered = std.numeric.Int64(fromBytes: bytes);
            if recovered.isNone() { return 2 }
            if recovered.unwrap() != 42 { return 3 }

            // Round-trip with negative value
            let negVal: std.numeric.Int64 = -999;
            let negBytes = negVal.toBytes();
            let negRecovered = std.numeric.Int64(fromBytes: negBytes);
            if negRecovered.isNone() { return 4 }
            if negRecovered.unwrap() != -999 { return 5 }

            // fromBytes with wrong number of bytes returns None
            var shortBytes = std.collections.Array[std.numeric.UInt8]();
            shortBytes.append(1);
            shortBytes.append(2);
            shortBytes.append(3);
            let shortResult = std.numeric.Int64(fromBytes: shortBytes);
            if shortResult.isSome() { return 6 }

            // fromBytes with empty array returns None
            let emptyBytes = std.collections.Array[std.numeric.UInt8]();
            let emptyResult = std.numeric.Int64(fromBytes: emptyBytes);
            if emptyResult.isSome() { return 7 }

            // Round-trip with zero
            let zeroVal: std.numeric.Int64 = 0;
            let zeroBytes = zeroVal.toBytes();
            let zeroRecovered = std.numeric.Int64(fromBytes: zeroBytes);
            if zeroRecovered.isNone() { return 8 }
            if zeroRecovered.unwrap() != 0 { return 9 }

            0
        }
