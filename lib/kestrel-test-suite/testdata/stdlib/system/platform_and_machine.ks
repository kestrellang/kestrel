// test: execution
// stdlib: true

// Platform.current and Machine report sane values for the host.

module Test

@main
func main() -> lang.i64 {
    let p = Platform.current;
    if p.pointerWidth != 64 and p.pointerWidth != 32 { return 1 }
    if p.architecture.name.isEmpty { return 2 }
    if not p.isEqual(to: Platform.current) { return 3 }
    if Machine.processorCount < 1 { return 4 }
    if Machine.pageSize < 4096 { return 5 }
    match Machine.hostName() {
        .Ok(name) => { if name.isEmpty { return 6 } },
        .Err(_) => { return 7 }
    }
    0
}
