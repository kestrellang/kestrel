// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Create data for cursor
            let byte10: std.numeric.UInt8 = 10;
            let byte20: std.numeric.UInt8 = 20;
            let byte30: std.numeric.UInt8 = 30;
            let byte40: std.numeric.UInt8 = 40;
            let byte50: std.numeric.UInt8 = 50;
            let zeroByte: std.numeric.UInt8 = 0;
            var data = std.collections.Array[std.numeric.UInt8]();
            data.append(byte10);
            data.append(byte20);
            data.append(byte30);
            data.append(byte40);
            data.append(byte50);

            // Create cursor
            var cursor = std.io.read.Cursor(data: data);

            // Initial position should be 0
            if cursor.position != 0 { return 1 }

            // Read first 3 bytes
            var buf = std.collections.Array[std.numeric.UInt8]();
            buf.append(zeroByte);
            buf.append(zeroByte);
            buf.append(zeroByte);
            let slice = std.memory.Slice[std.numeric.UInt8](pointer: buf.asPointer(), count: 3);
            let result = cursor.read(into: slice);
            match result {
                .Ok(n) => if n != 3 { return 2 },
                .Err(_) => return 3
            }

            // Verify bytes read
            if buf(unchecked: 0) != byte10 { return 4 }
            if buf(unchecked: 1) != byte20 { return 5 }
            if buf(unchecked: 2) != byte30 { return 6 }

            // Position should be 3
            if cursor.position != 3 { return 7 }

            // Read remaining 2 bytes (request 5 but only 2 available)
            var buf2 = std.collections.Array[std.numeric.UInt8]();
            buf2.append(zeroByte);
            buf2.append(zeroByte);
            buf2.append(zeroByte);
            buf2.append(zeroByte);
            buf2.append(zeroByte);
            let slice2 = std.memory.Slice[std.numeric.UInt8](pointer: buf2.asPointer(), count: 5);
            let result2 = cursor.read(into: slice2);
            match result2 {
                .Ok(n) => if n != 2 { return 8 },
                .Err(_) => return 9
            }
            if buf2(unchecked: 0) != byte40 { return 10 }
            if buf2(unchecked: 1) != byte50 { return 11 }

            // Position should be 5
            if cursor.position != 5 { return 12 }

            // Read at EOF should return 0
            let result3 = cursor.read(into: slice);
            match result3 {
                .Ok(n) => if n != 0 { return 13 },
                .Err(_) => return 14
            }

            // Test setPosition
            cursor.setPosition(to: 1);
            if cursor.position != 1 { return 15 }

            // Read after setPosition
            var buf3 = std.collections.Array[std.numeric.UInt8]();
            buf3.append(zeroByte);
            buf3.append(zeroByte);
            let slice3 = std.memory.Slice[std.numeric.UInt8](pointer: buf3.asPointer(), count: 2);
            let result4 = cursor.read(into: slice3);
            match result4 {
                .Ok(n) => if n != 2 { return 16 },
                .Err(_) => return 17
            }
            if buf3(unchecked: 0) != byte20 { return 18 }
            if buf3(unchecked: 1) != byte30 { return 19 }

            // setPosition clamps negative to 0
            cursor.setPosition(to: -5);
            if cursor.position != 0 { return 20 }

            // setPosition clamps beyond end to data count
            cursor.setPosition(to: 100);
            if cursor.position != 5 { return 21 }

            0
        }
