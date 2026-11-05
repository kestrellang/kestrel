// test: execution
// stdlib: true

// Process reports this process's id, parent, executable and argv.

module Test

@main
func main() -> lang.i64 {
    if Process.id.value <= 0 { return 1 }
    if Process.parentId.value <= 0 { return 2 }
    if Process.id.isEqual(to: Process.parentId) { return 3 }
    match Process.executablePath() {
        .Ok(path) => { if not path.starts(with: "/") { return 4 } },
        .Err(_) => { return 5 }
    }
    // argv[0] is always there.
    let args = Process.arguments();
    if args.count < 1 { return 6 }
    0
}
