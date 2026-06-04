// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Create a Repeat reader that yields byte 42
            let byte42: std.numeric.UInt8 = 42;
            var rep = std.io.read.Repeat(byte: byte42);

            // Create a buffer to read into
            let zeroByte: std.numeric.UInt8 = 0;
            var buf = std.collections.Array[std.numeric.UInt8]();
            buf.append(zeroByte);
            buf.append(zeroByte);
            buf.append(zeroByte);
            buf.append(zeroByte);
            buf.append(zeroByte);
            let slice = std.memory.ArraySlice[std.numeric.UInt8](pointer: buf.asPointer(), count: 5);

            // Read should fill the entire buffer with byte 42
            let result = rep.read(into: slice);
            match result {
                .Ok(n) => if n != 5 { return 1 },
                .Err(_) => return 2
            }

            // Verify all bytes are 42
            if buf(unchecked: 0) != byte42 { return 3 }
            if buf(unchecked: 1) != byte42 { return 4 }
            if buf(unchecked: 2) != byte42 { return 5 }
            if buf(unchecked: 3) != byte42 { return 6 }
            if buf(unchecked: 4) != byte42 { return 7 }

            // Read again with a different size buffer
            var buf2 = std.collections.Array[std.numeric.UInt8]();
            buf2.append(zeroByte);
            buf2.append(zeroByte);
            let slice2 = std.memory.ArraySlice[std.numeric.UInt8](pointer: buf2.asPointer(), count: 2);
            let result2 = rep.read(into: slice2);
            match result2 {
                .Ok(n) => if n != 2 { return 8 },
                .Err(_) => return 9
            }
            if buf2(unchecked: 0) != byte42 { return 10 }
            if buf2(unchecked: 1) != byte42 { return 11 }

            0
        }
