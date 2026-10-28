// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Formattable delegates to description() for named kinds
            let nf = std.io.error.notFound();
            if not nf.formatted().isEqual(to: "no such file or directory") { return 1 }

            // Interpolation goes through Formattable too
            let msg = "err: \(nf)";
            if not msg.isEqual(to: "err: no such file or directory") { return 2 }

            // .Other includes the raw errno in the rendered form
            let code999: std.numeric.Int32 = 999;
            let unk = std.io.error.IoError(code: code999);
            if not unk.formatted().isEqual(to: "unknown error (errno 999)") { return 3 }

            // Width/alignment options are honored via _writePadded
            let padded = "\(nf:>30)";
            if padded.chars.count != 30 { return 4 }

            0
        }
