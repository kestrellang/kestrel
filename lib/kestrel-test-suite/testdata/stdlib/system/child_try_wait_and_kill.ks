// test: execution
// stdlib: true

// tryWait doesn't block, a timed wait gives up, and kill ends the child.

module Test

@main
func main() -> lang.i64 {
    var child = match Command("sleep").argument("10").start() {
        .Ok(c) => c,
        .Err(_) => { return 1 }
    };
    if child.id.value <= 0 { return 2 }
    match child.tryWait() {
        .Ok(.None) => {},
        _ => { return 3 }
    }
    match child.wait(milliseconds: 20) {
        .Ok(.None) => {},
        _ => { return 4 }
    }
    match child.kill() {
        .Ok(_) => {},
        .Err(_) => { return 5 }
    }
    match child.wait() {
        .Ok(status) => {
            if not status.isEqual(to: .Terminated(signal: 9)) { return 6 }
            if status.code.isSome { return 7 }
        },
        .Err(_) => { return 8 }
    }
    // The status is remembered; a second wait returns it again.
    match child.wait() {
        .Ok(status) => { if not status.isEqual(to: .Terminated(signal: 9)) { return 9 } },
        .Err(_) => { return 10 }
    }
    0
}
