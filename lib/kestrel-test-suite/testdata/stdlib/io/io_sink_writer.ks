// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Create a Sink writer
            var sink = std.io.write.Sink();

            // Create data to write
            let byte1: std.numeric.UInt8 = 1;
            let byte2: std.numeric.UInt8 = 2;
            let byte3: std.numeric.UInt8 = 3;
            var data = std.collections.Array[std.numeric.UInt8]();
            data.append(byte1);
            data.append(byte2);
            data.append(byte3);
            let slice = std.memory.Slice[std.numeric.UInt8](pointer: data.asPointer(), count: 3);

            // Write should succeed and report all bytes as written
            let result = sink.write(from: slice);
            match result {
                .Ok(n) => if n != 3 { return 1 },
                .Err(_) => return 2
            }

            // Flush should succeed
            let flushResult = sink.flush();
            match flushResult {
                .Ok(_) => 0,
                .Err(_) => return 3
            }

            // Write with a larger buffer
            let byte255: std.numeric.UInt8 = 255;
            var big = std.collections.Array[std.numeric.UInt8]();
            var i: std.numeric.Int64 = 0;
            while i < 100 {
                big.append(byte255);
                i = i + 1
            }
            let bigSlice = std.memory.Slice[std.numeric.UInt8](pointer: big.asPointer(), count: 100);
            let result2 = sink.write(from: bigSlice);
            match result2 {
                .Ok(n) => if n != 100 { return 4 },
                .Err(_) => return 5
            }

            0
        }
