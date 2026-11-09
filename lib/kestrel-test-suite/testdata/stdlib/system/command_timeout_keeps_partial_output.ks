// test: execution
// stdlib: true
// expect-stdout: timed out\nbefore\noops\nfalse\n

// When a command outlives its timeout, `output()` still returns what the
// child wrote before it was killed, with status `.TimedOut`.

module Test

@main
func main() -> lang.i64 {
    let script = "echo before; echo oops >&2; sleep 10";
    match Command("sh").arguments(["-c", script]).timeout(.milliseconds(500)).output() {
        .Ok(out) => {
            println(out.status);
            print(out.standardOutputText() ?? "<not utf-8>");
            print(out.standardErrorText() ?? "<not utf-8>");
            println(out.status.isSuccess);
        },
        .Err(_) => { return 1 }
    }
    0
}
