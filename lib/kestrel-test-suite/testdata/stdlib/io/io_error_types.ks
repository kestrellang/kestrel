// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test IoError constructor from raw errno
            let code2: std.num.Int32 = 2;
            let err = std.io.error.IoError(code: code2);
            if err.errno() != code2 { return 1 }

            // Test description for known codes
            let desc = err.description();
            if not desc.equals("no such file or directory") { return 2 }

            // Test notFound() convenience constructor
            let nf = std.io.error.notFound();
            if nf.errno() != code2 { return 3 }
            if not nf.description().equals("no such file or directory") { return 4 }

            // Test permissionDenied()
            let pd = std.io.error.permissionDenied();
            let code13: std.num.Int32 = 13;
            if pd.errno() != code13 { return 5 }
            if not pd.description().equals("permission denied") { return 6 }

            // Test alreadyExists()
            let ae = std.io.error.alreadyExists();
            let code17: std.num.Int32 = 17;
            if ae.errno() != code17 { return 7 }
            if not ae.description().equals("file exists") { return 8 }

            // Test invalidInput()
            let ii = std.io.error.invalidInput();
            let code22: std.num.Int32 = 22;
            if ii.errno() != code22 { return 9 }
            if not ii.description().equals("invalid argument") { return 10 }

            // Test wouldBlock()
            let wb = std.io.error.wouldBlock();
            let code11: std.num.Int32 = 11;
            if wb.errno() != code11 { return 11 }
            if not wb.description().equals("would block") { return 12 }

            // Test interrupted()
            let intr = std.io.error.interrupted();
            let code4: std.num.Int32 = 4;
            if intr.errno() != code4 { return 13 }
            if not intr.description().equals("interrupted") { return 14 }

            // Test brokenPipe()
            let bp = std.io.error.brokenPipe();
            let code32: std.num.Int32 = 32;
            if bp.errno() != code32 { return 15 }
            if not bp.description().equals("broken pipe") { return 16 }

            // Test unknown error code
            let code999: std.num.Int32 = 999;
            let unk = std.io.error.IoError(code: code999);
            if not unk.description().equals("unknown error") { return 17 }

            0
        }
