// test: execution
// stdlib: true

// Command.find searches PATH the way spawning does.

module Test

@main
func main() -> lang.i64 {
    match Command.find("sh") {
        .Some(path) => { if not path.ends(with: "/sh") { return 1 } },
        .None => { return 2 }
    }
    if Command.find("kestrel-no-such-program-xyz").isSome { return 3 }
    // A path is checked as-is.
    if Command.find("/bin/sh").isNone { return 4 }
    if Command.find("/definitely/not/sh").isSome { return 5 }
    0
}
