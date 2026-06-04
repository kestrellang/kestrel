// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test readByte using a Cursor
            let byte65: std.numeric.UInt8 = 65; // 'A'
            let byte66: std.numeric.UInt8 = 66; // 'B'
            let byte67: std.numeric.UInt8 = 67; // 'C'
            var data = std.collections.Array[std.numeric.UInt8]();
            data.append(byte65);
            data.append(byte66);
            data.append(byte67);

            var cursor = std.io.read.Cursor(data: data);

            // readByte should return first byte
            let rb1 = std.io.read.readByte( cursor);
            match rb1 {
                .Ok(opt) => match opt {
                    .Some(b) => if b != byte65 { return 1 },
                    .None => return 2
                },
                .Err(_) => return 3
            }

            // readByte should return second byte
            let rb2 = std.io.read.readByte( cursor);
            match rb2 {
                .Ok(opt) => match opt {
                    .Some(b) => if b != byte66 { return 4 },
                    .None => return 5
                },
                .Err(_) => return 6
            }

            // readByte should return third byte
            let rb3 = std.io.read.readByte( cursor);
            match rb3 {
                .Ok(opt) => match opt {
                    .Some(b) => if b != byte67 { return 7 },
                    .None => return 8
                },
                .Err(_) => return 9
            }

            // readByte at EOF should return None
            let rb4 = std.io.read.readByte( cursor);
            match rb4 {
                .Ok(opt) => match opt {
                    .Some(_) => return 10,
                    .None => 0
                },
                .Err(_) => return 11
            }

            // Test readAll using a Cursor
            let byte1: std.numeric.UInt8 = 1;
            let byte2: std.numeric.UInt8 = 2;
            let byte3: std.numeric.UInt8 = 3;
            var data2 = std.collections.Array[std.numeric.UInt8]();
            data2.append(byte1);
            data2.append(byte2);
            data2.append(byte3);
            var cursor2 = std.io.read.Cursor(data: data2);
            var dest = std.collections.Array[std.numeric.UInt8]();
            let raResult = std.io.read.readAll( cursor2, into: dest);
            match raResult {
                .Ok(n) => if n != 3 { return 12 },
                .Err(_) => return 13
            }
            if dest.count != 3 { return 14 }
            if dest(unchecked: 0) != byte1 { return 15 }
            if dest(unchecked: 1) != byte2 { return 16 }
            if dest(unchecked: 2) != byte3 { return 17 }

            // Test readAll on Empty reader
            var empty = std.io.read.Empty();
            var emptyDest = std.collections.Array[std.numeric.UInt8]();
            let raEmpty = std.io.read.readAll( empty, into: emptyDest);
            match raEmpty {
                .Ok(n) => if n != 0 { return 18 },
                .Err(_) => return 19
            }
            if emptyDest.count != 0 { return 20 }

            0
        }
