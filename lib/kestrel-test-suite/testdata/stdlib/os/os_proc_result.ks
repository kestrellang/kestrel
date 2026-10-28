// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // spawn returns Ok(exitCode); a non-zero exit is still Ok
            match std.os.spawn("exit 7") {
                .Ok(code) => {
                    let expected: std.numeric.Int32 = 7;
                    if code != expected { return 1 }
                },
                .Err(_) => { return 2 }
            }

            // captureOutput returns Ok with trailing newline chomped
            match std.os.captureOutput("echo hello") {
                .Ok(out) => { if not out.isEqual(to: "hello") { return 3 } },
                .Err(_) => { return 4 }
            }

            0
        }
