// test: execution
// stdlib: true
// expect-stdout: [out\n][err\n] exit code 3\n

// output() captures stdout and stderr separately along with the status.

module Test

@main
func main() -> lang.i64 {
    match Command("sh").arguments(["-c", "echo out; echo err >&2; exit 3"]).output() {
        .Ok(o) => {
            let out = o.standardOutputText() ?? "?";
            let err = o.standardErrorText() ?? "?";
            println("[\(out)][\(err)] \(o.status)");
        },
        .Err(_) => { return 1 }
    }
    0
}
