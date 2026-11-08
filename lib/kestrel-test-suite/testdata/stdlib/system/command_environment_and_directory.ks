// test: execution
// stdlib: true

// Per-child environment changes and working directory, without touching the
// parent's.

module Test

func shellOutput(command: Command) -> String {
    match command.output() {
        .Ok(o) => o.standardOutputText() ?? "<not utf8>",
        .Err(_) => "<error>"
    }
}

@main
func main() -> lang.i64 {
    var withVar = Command("sh");
    withVar.arguments(["-c", "printf %s \"$KESTREL_CHILD_VAR\""]).environment("KESTREL_CHILD_VAR", to: "child");
    if not shellOutput(withVar).isEqual(to: "child") { return 1 }
    // The parent never saw it.
    if Environment.hasVariable("KESTREL_CHILD_VAR") { return 2 }

    let _ = Environment.setVariable("KESTREL_PARENT_VAR", to: "parent");
    var inherit = Command("sh");
    inherit.arguments(["-c", "printf %s \"$KESTREL_PARENT_VAR\""]);
    if not shellOutput(inherit).isEqual(to: "parent") { return 3 }

    var removed = Command("sh");
    removed.arguments(["-c", "printf %s \"${KESTREL_PARENT_VAR-unset}\""]).removeEnvironment("KESTREL_PARENT_VAR");
    if not shellOutput(removed).isEqual(to: "unset") { return 4 }

    var cleared = Command("/bin/sh");
    cleared.arguments(["-c", "printf %s \"${KESTREL_PARENT_VAR-unset}:$ONLY\""]).clearEnvironment().environment("ONLY", to: "1");
    if not shellOutput(cleared).isEqual(to: "unset:1") { return 5 }

    var moved = Command("pwd");
    moved.directory("/");
    if not shellOutput(moved).isEqual(to: "/\n") { return 6 }
    0
}
