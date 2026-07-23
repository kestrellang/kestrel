// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // getcwd now returns Result; success yields a non-empty path
            match std.os.getcwd() {
                .Ok(cwd) => { if cwd.isEmpty { return 1 } },
                .Err(_) => { return 2 }
            }

            // listDir on the cwd succeeds (Ok, possibly empty)
            match std.os.listDir(".") {
                .Ok(_) => {},
                .Err(_) => { return 3 }
            }

            // listDir on a missing path surfaces the errno as NotFound
            match std.os.listDir("/definitely/not/a/real/dir-kestrel-test") {
                .Ok(_) => { return 4 },
                .Err(e) => {
                    match e.kind {
                        .NotFound => {},
                        _ => { return 5 }
                    }
                }
            }

            0
        }
