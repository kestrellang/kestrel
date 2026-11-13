// test: execution
// stdlib: true
// expect-stdout: 3\n

// Command.shell is the explicit opt-in to /bin/sh parsing.

module Test

@main
func main() -> lang.i64 {
    match Command.shell("echo $((1 + 2))").run() {
        .Ok(status) => { if not status.isSuccess { return 1 } },
        .Err(_) => { return 2 }
    }
    0
}
