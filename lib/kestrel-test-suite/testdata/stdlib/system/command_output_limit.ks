// test: execution
// stdlib: true

// output(limit:) keeps at most `limit` bytes per stream and still drains the
// rest, so the child isn't left blocked on a full pipe.

module Test

@main
func main() -> lang.i64 {
    match Command("head").arguments(["-c", "200000", "/dev/zero"]).output(limit: 10) {
        .Ok(o) => {
            if not o.status.isSuccess { return 1 }
            if o.standardOutput.count != 10 { return 2 }
        },
        .Err(_) => { return 3 }
    }
    // Without a limit, everything comes back.
    match Command("head").arguments(["-c", "200000", "/dev/zero"]).output() {
        .Ok(o) => { if o.standardOutput.count != 200000 { return 4 } },
        .Err(_) => { return 5 }
    }
    0
}
