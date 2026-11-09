// test: execution
// stdlib: true
// expect-stdout: terminated by signal 15\ntrue true false\n

module Test

// A child ended by a signal reports which one: `terminate()` sends SIGTERM
// (15), so the status is `.Terminated(signal: 15)`, which is not equal to a
// SIGKILL (9) termination and has no exit code.

@main
func main() -> lang.i64 {
    var child = match Command("sleep").argument("10").start() {
        .Ok(c) => c,
        .Err(_) => { return 1 }
    };
    match child.terminate() {
        .Ok(_) => {},
        .Err(_) => { return 2 }
    }
    match child.wait() {
        .Ok(status) => {
            print("\(status)\n");
            let signal = match status {
                .Terminated(signal: s) => s == 15,
                .Exited(_) => false,
                .TimedOut => false
            };
            print("\(signal) \(status.code.isNone) \(status.isEqual(to: .Terminated(signal: 9)))\n");
        },
        .Err(_) => { return 3 }
    }
    0
}
