// test: execution
// stdlib: true

// A command that outlives its timeout is killed; the call still succeeds,
// with status `.TimedOut`.

module Test

@main
func main() -> lang.i64 {
    let ran = Command("sleep").argument("10").timeout(milliseconds: 50).run();
    match ran {
        .Ok(status) => { if not status.isEqual(to: .TimedOut) { return 1 } },
        .Err(_) => { return 1 }
    }
    let captured = Command("sleep").argument("10").timeout(milliseconds: 50).output();
    match captured {
        .Ok(out) => { if not out.status.isEqual(to: .TimedOut) or out.status.isSuccess { return 2 } },
        .Err(_) => { return 2 }
    }
    // A command that finishes in time is unaffected.
    let quick = Command("true").timeout(milliseconds: 5000).run();
    match quick {
        .Ok(status) => { if not status.isSuccess { return 3 } },
        .Err(_) => { return 4 }
    }
    0
}
