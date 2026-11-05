// test: execution
// stdlib: true

// currentDirectory follows setCurrentDirectory; home and temp directories
// resolve to something.

module Test

@main
func main() -> lang.i64 {
    match Environment.setCurrentDirectory(to: "/") {
        .Ok(_) => {},
        .Err(_) => { return 1 }
    }
    match Environment.currentDirectory() {
        .Ok(dir) => { if not dir.isEqual(to: "/") { return 2 } },
        .Err(_) => { return 3 }
    }
    // A missing directory is an error, and the working directory stays put.
    match Environment.setCurrentDirectory(to: "/definitely/not/a/directory") {
        .Ok(_) => { return 4 },
        .Err(e) => { match e.kind { .NotFound => {}, _ => { return 5 } } }
    }

    let _ = Environment.setVariable("TMPDIR", to: "/var/tmp-kestrel");
    if not Environment.temporaryDirectory().isEqual(to: "/var/tmp-kestrel") { return 6 }
    let _ = Environment.removeVariable("TMPDIR");
    if not Environment.temporaryDirectory().isEqual(to: "/tmp") { return 7 }

    let _ = Environment.setVariable("HOME", to: "/home/kestrel-test");
    match Environment.homeDirectory() {
        .Some(home) => { if not home.isEqual(to: "/home/kestrel-test") { return 8 } },
        .None => { return 9 }
    }
    0
}
