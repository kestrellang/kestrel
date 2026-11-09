// test: execution
// stdlib: true
// expect-stdout: 3\nkey\nvalue\nkey\n7\n

// Every byte position std hands out for a slice counts from the slice's
// own first byte: `firstIndex(of:)`, `lastIndex(of:)`, `subslice` and the
// `bytes` view agree, so an index from one feeds the others. (Before,
// `firstIndex` counted from the parent string and `bytes` from the slice,
// so this printed `key=v`.)

module Test

@main
func main() {
    let s = "xxkey=value";
    let all = s.asSlice();
    let sl = all.subslice(from: 2, to: all.bytes.count);
    guard let .Some(eq) = sl.firstIndex(of: "=") else { return; }
    println(eq.value);
    println(sl.subslice(from: 0, to: eq.value));
    println(sl.subslice(from: eq.value + 1, to: sl.bytes.count));
    println(sl.bytes.slice(from: sl.bytes.startIndex, to: eq));
    guard let .Some(last) = sl.lastIndex(of: "u") else { return; }
    println(last.value);
}
