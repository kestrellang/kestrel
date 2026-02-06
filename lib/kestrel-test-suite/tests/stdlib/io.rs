use kestrel_test_suite::*;

#[test]
fn io_error_types() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test Error constructor with code
            let code2: std.num.Int32 = 2;
            let err = std.io.error.Error( code2);
            if err.errno() != code2 { return 1 }

            // Test description for known codes
            let desc = err.description();
            if desc.equals("no such file or directory") == false { return 2 }

            // Test notFound() convenience constructor
            let nf = std.io.error.notFound();
            if nf.errno() != code2 { return 3 }
            if nf.description().equals("no such file or directory") == false { return 4 }

            // Test permissionDenied()
            let pd = std.io.error.permissionDenied();
            let code13: std.num.Int32 = 13;
            if pd.errno() != code13 { return 5 }
            if pd.description().equals("permission denied") == false { return 6 }

            // Test alreadyExists()
            let ae = std.io.error.alreadyExists();
            let code17: std.num.Int32 = 17;
            if ae.errno() != code17 { return 7 }
            if ae.description().equals("file exists") == false { return 8 }

            // Test invalidInput()
            let ii = std.io.error.invalidInput();
            let code22: std.num.Int32 = 22;
            if ii.errno() != code22 { return 9 }
            if ii.description().equals("invalid argument") == false { return 10 }

            // Test wouldBlock()
            let wb = std.io.error.wouldBlock();
            let code11: std.num.Int32 = 11;
            if wb.errno() != code11 { return 11 }
            if wb.description().equals("would block") == false { return 12 }

            // Test interrupted()
            let intr = std.io.error.interrupted();
            let code4: std.num.Int32 = 4;
            if intr.errno() != code4 { return 13 }
            if intr.description().equals("interrupted") == false { return 14 }

            // Test brokenPipe()
            let bp = std.io.error.brokenPipe();
            let code32: std.num.Int32 = 32;
            if bp.errno() != code32 { return 15 }
            if bp.description().equals("broken pipe") == false { return 16 }

            // Test unknown error code
            let code999: std.num.Int32 = 999;
            let unk = std.io.error.Error( code999);
            if unk.description().equals("unknown error") == false { return 17 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn io_empty_reader() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create an Empty reader
            var empty = std.io.read.Empty();

            // Create a buffer to read into
            let zeroByte: std.num.UInt8 = 0;
            var buf = std.collections.Array[std.num.UInt8]();
            buf.append(zeroByte);
            buf.append(zeroByte);
            buf.append(zeroByte);
            let slice = std.memory.Slice[std.num.UInt8](pointer: buf.asPointer(), count: 3);

            // Read should return Ok(0) immediately (EOF)
            let result = empty.read(into: slice);
            match result {
                .Ok(n) => if n != 0 { return 1 },
                .Err(_) => return 2
            }

            // Read again should still return Ok(0)
            let result2 = empty.read(into: slice);
            match result2 {
                .Ok(n) => if n != 0 { return 3 },
                .Err(_) => return 4
            }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn io_repeat_reader() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create a Repeat reader that yields byte 42
            let byte42: std.num.UInt8 = 42;
            var rep = std.io.read.Repeat( byte42);

            // Create a buffer to read into
            let zeroByte: std.num.UInt8 = 0;
            var buf = std.collections.Array[std.num.UInt8]();
            buf.append(zeroByte);
            buf.append(zeroByte);
            buf.append(zeroByte);
            buf.append(zeroByte);
            buf.append(zeroByte);
            let slice = std.memory.Slice[std.num.UInt8](pointer: buf.asPointer(), count: 5);

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
            var buf2 = std.collections.Array[std.num.UInt8]();
            buf2.append(zeroByte);
            buf2.append(zeroByte);
            let slice2 = std.memory.Slice[std.num.UInt8](pointer: buf2.asPointer(), count: 2);
            let result2 = rep.read(into: slice2);
            match result2 {
                .Ok(n) => if n != 2 { return 8 },
                .Err(_) => return 9
            }
            if buf2(unchecked: 0) != byte42 { return 10 }
            if buf2(unchecked: 1) != byte42 { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn io_cursor() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create data for cursor
            let byte10: std.num.UInt8 = 10;
            let byte20: std.num.UInt8 = 20;
            let byte30: std.num.UInt8 = 30;
            let byte40: std.num.UInt8 = 40;
            let byte50: std.num.UInt8 = 50;
            let zeroByte: std.num.UInt8 = 0;
            var data = std.collections.Array[std.num.UInt8]();
            data.append(byte10);
            data.append(byte20);
            data.append(byte30);
            data.append(byte40);
            data.append(byte50);

            // Create cursor
            var cursor = std.io.read.Cursor( data);

            // Initial position should be 0
            if cursor.position() != 0 { return 1 }

            // Read first 3 bytes
            var buf = std.collections.Array[std.num.UInt8]();
            buf.append(zeroByte);
            buf.append(zeroByte);
            buf.append(zeroByte);
            let slice = std.memory.Slice[std.num.UInt8](pointer: buf.asPointer(), count: 3);
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
            if cursor.position() != 3 { return 7 }

            // Read remaining 2 bytes (request 5 but only 2 available)
            var buf2 = std.collections.Array[std.num.UInt8]();
            buf2.append(zeroByte);
            buf2.append(zeroByte);
            buf2.append(zeroByte);
            buf2.append(zeroByte);
            buf2.append(zeroByte);
            let slice2 = std.memory.Slice[std.num.UInt8](pointer: buf2.asPointer(), count: 5);
            let result2 = cursor.read(into: slice2);
            match result2 {
                .Ok(n) => if n != 2 { return 8 },
                .Err(_) => return 9
            }
            if buf2(unchecked: 0) != byte40 { return 10 }
            if buf2(unchecked: 1) != byte50 { return 11 }

            // Position should be 5
            if cursor.position() != 5 { return 12 }

            // Read at EOF should return 0
            let result3 = cursor.read(into: slice);
            match result3 {
                .Ok(n) => if n != 0 { return 13 },
                .Err(_) => return 14
            }

            // Test setPosition
            cursor.setPosition(to: 1);
            if cursor.position() != 1 { return 15 }

            // Read after setPosition
            var buf3 = std.collections.Array[std.num.UInt8]();
            buf3.append(zeroByte);
            buf3.append(zeroByte);
            let slice3 = std.memory.Slice[std.num.UInt8](pointer: buf3.asPointer(), count: 2);
            let result4 = cursor.read(into: slice3);
            match result4 {
                .Ok(n) => if n != 2 { return 16 },
                .Err(_) => return 17
            }
            if buf3(unchecked: 0) != byte20 { return 18 }
            if buf3(unchecked: 1) != byte30 { return 19 }

            // setPosition clamps negative to 0
            cursor.setPosition(to: -5);
            if cursor.position() != 0 { return 20 }

            // setPosition clamps beyond end to data count
            cursor.setPosition(to: 100);
            if cursor.position() != 5 { return 21 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn io_sink_writer() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create a Sink writer
            var sink = std.io.write.Sink();

            // Create data to write
            let byte1: std.num.UInt8 = 1;
            let byte2: std.num.UInt8 = 2;
            let byte3: std.num.UInt8 = 3;
            var data = std.collections.Array[std.num.UInt8]();
            data.append(byte1);
            data.append(byte2);
            data.append(byte3);
            let slice = std.memory.Slice[std.num.UInt8](pointer: data.asPointer(), count: 3);

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
            let byte255: std.num.UInt8 = 255;
            var big = std.collections.Array[std.num.UInt8]();
            var i: std.num.Int64 = 0;
            while i < 100 {
                big.append(byte255);
                i = i + 1
            }
            let bigSlice = std.memory.Slice[std.num.UInt8](pointer: big.asPointer(), count: 100);
            let result2 = sink.write(from: bigSlice);
            match result2 {
                .Ok(n) => if n != 100 { return 4 },
                .Err(_) => return 5
            }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn io_buffer_writer() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create a Buffer writer
            var buf = std.io.write.Buffer();

            // Initially empty
            if buf.isEmpty() == false { return 1 }
            if buf.count() != 0 { return 2 }

            // Write some bytes
            let byte72: std.num.UInt8 = 72;   // 'H'
            let byte101: std.num.UInt8 = 101; // 'e'
            let byte108: std.num.UInt8 = 108; // 'l'
            let byte111: std.num.UInt8 = 111; // 'o'
            var data = std.collections.Array[std.num.UInt8]();
            data.append(byte72);
            data.append(byte101);
            data.append(byte108);
            data.append(byte108);
            data.append(byte111);
            let slice = std.memory.Slice[std.num.UInt8](pointer: data.asPointer(), count: 5);
            let result = buf.write(from: slice);
            match result {
                .Ok(n) => if n != 5 { return 3 },
                .Err(_) => return 4
            }

            // Check count
            if buf.count() != 5 { return 5 }
            if buf.isEmpty() { return 6 }

            // Check toString
            let s = buf.toString();
            if s.equals("Hello") == false { return 7 }

            // Check toArray
            let arr = buf.toArray();
            if arr.count != 5 { return 8 }
            if arr(unchecked: 0) != byte72 { return 9 }

            // Check asSlice
            let sl = buf.asSlice();
            if sl.count != 5 { return 10 }
            if sl(unchecked: 0) != byte72 { return 11 }

            // Flush should succeed (no-op for Buffer)
            let flushResult = buf.flush();
            match flushResult {
                .Ok(_) => 0,
                .Err(_) => return 12
            }

            // Write more data
            let byte33: std.num.UInt8 = 33; // '!'
            var data2 = std.collections.Array[std.num.UInt8]();
            data2.append(byte33);
            let slice2 = std.memory.Slice[std.num.UInt8](pointer: data2.asPointer(), count: 1);
            let result2 = buf.write(from: slice2);
            match result2 {
                .Ok(n) => if n != 1 { return 13 },
                .Err(_) => return 14
            }
            if buf.count() != 6 { return 15 }
            if buf.toString().equals("Hello!") == false { return 16 }

            // Test clear
            buf.clear();
            if buf.count() != 0 { return 17 }
            if buf.isEmpty() == false { return 18 }

            // Test init with capacity
            var buf2 = std.io.write.Buffer( 64);
            if buf2.isEmpty() == false { return 19 }
            if buf2.count() != 0 { return 20 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn io_read_helpers() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test readByte using a Cursor
            let byte65: std.num.UInt8 = 65; // 'A'
            let byte66: std.num.UInt8 = 66; // 'B'
            let byte67: std.num.UInt8 = 67; // 'C'
            var data = std.collections.Array[std.num.UInt8]();
            data.append(byte65);
            data.append(byte66);
            data.append(byte67);

            var cursor = std.io.read.Cursor( data);

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
            let byte1: std.num.UInt8 = 1;
            let byte2: std.num.UInt8 = 2;
            let byte3: std.num.UInt8 = 3;
            var data2 = std.collections.Array[std.num.UInt8]();
            data2.append(byte1);
            data2.append(byte2);
            data2.append(byte3);
            var cursor2 = std.io.read.Cursor( data2);
            var dest = std.collections.Array[std.num.UInt8]();
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
            var emptyDest = std.collections.Array[std.num.UInt8]();
            let raEmpty = std.io.read.readAll( empty, into: emptyDest);
            match raEmpty {
                .Ok(n) => if n != 0 { return 18 },
                .Err(_) => return 19
            }
            if emptyDest.count != 0 { return 20 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn io_write_helpers() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test writeByte using Buffer
            var buf = std.io.write.Buffer();
            let byte65: std.num.UInt8 = 65;
            let wb1 = std.io.write.writeByte( buf, byte65);
            match wb1 {
                .Ok(_) => 0,
                .Err(_) => return 1
            }
            if buf.count() != 1 { return 2 }

            // Test writeStr using Buffer
            var buf2 = std.io.write.Buffer();
            let ws = std.io.write.writeStr( buf2, "Hello");
            match ws {
                .Ok(_) => 0,
                .Err(_) => return 3
            }
            if buf2.count() != 5 { return 4 }
            if buf2.toString().equals("Hello") == false { return 5 }

            // Test writeLine using Buffer
            var buf3 = std.io.write.Buffer();
            let wl = std.io.write.writeLine( buf3, "Hi");
            match wl {
                .Ok(_) => 0,
                .Err(_) => return 6
            }
            // "Hi" + newline = 3 bytes
            if buf3.count() != 3 { return 7 }

            // Test writeAll using Buffer
            var buf4 = std.io.write.Buffer();
            let byte1: std.num.UInt8 = 1;
            let byte2: std.num.UInt8 = 2;
            let byte3: std.num.UInt8 = 3;
            var data = std.collections.Array[std.num.UInt8]();
            data.append(byte1);
            data.append(byte2);
            data.append(byte3);
            let slice = std.memory.Slice[std.num.UInt8](pointer: data.asPointer(), count: 3);
            let wa = std.io.write.writeAll( buf4, from: slice);
            match wa {
                .Ok(_) => 0,
                .Err(_) => return 8
            }
            if buf4.count() != 3 { return 9 }
            let arr = buf4.toArray();
            if arr(unchecked: 0) != byte1 { return 10 }
            if arr(unchecked: 1) != byte2 { return 11 }
            if arr(unchecked: 2) != byte3 { return 12 }

            // Test writeStr with empty string
            var buf5 = std.io.write.Buffer();
            let wsEmpty = std.io.write.writeStr( buf5, "");
            match wsEmpty {
                .Ok(_) => 0,
                .Err(_) => return 13
            }
            if buf5.count() != 0 { return 14 }

            // Test multiple writes accumulate
            var buf6 = std.io.write.Buffer();
            let _ = std.io.write.writeStr( buf6, "Hello");
            let _ = std.io.write.writeStr( buf6, " ");
            let _ = std.io.write.writeStr( buf6, "World");
            if buf6.toString().equals("Hello World") == false { return 15 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
