// test: execution
// stdlib: true

// A command that outlives its timeout is killed and the call fails.

module Test

@main
func main() -> lang.i64 {
    let ran = Command("sleep").argument("10").timeout(milliseconds: 50).run();
    match ran {
        .Ok(_) => { return 1 },
        .Err(_) => {}
    }
    let captured = Command("sleep").argument("10").timeout(milliseconds: 50).output();
    match captured {
        .Ok(_) => { return 2 },
        .Err(_) => {}
    }
    // A command that finishes in time is unaffected.
    let quick = Command("true").timeout(milliseconds: 5000).run();
    match quick {
        .Ok(status) => { if not status.isSuccess { return 3 } },
        .Err(_) => { return 4 }
    }
    0
}
