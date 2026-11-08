// test: execution
// stdlib: true

// run() reports the child's exit code; a missing program is an error, not
// an exit status.

module Test

@main
func main() -> lang.i64 {
    match run("true", []) {
        .Ok(status) => { if not status.isSuccess { return 1 } },
        .Err(_) => { return 2 }
    }
    match run("false", []) {
        .Ok(status) => {
            if status.isSuccess { return 3 }
            if not status.isEqual(to: .Exited(1)) { return 4 }
        },
        .Err(_) => { return 5 }
    }
    match Command("sh").arguments(["-c", "exit 7"]).run() {
        .Ok(status) => {
            match status.code {
                .Some(code) => { if code.value != 7 { return 6 } },
                .None => { return 7 }
            }
        },
        .Err(_) => { return 8 }
    }
    match run("kestrel-no-such-program-xyz", []) {
        .Ok(_) => { return 9 },
        .Err(e) => { match e.kind { .NotFound => {}, _ => { return 10 } } }
    }
    0
}
