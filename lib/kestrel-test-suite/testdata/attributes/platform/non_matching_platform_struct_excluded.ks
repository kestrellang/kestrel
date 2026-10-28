// test: diagnostics
// stdlib: false
// host: darwin

// A struct gated on a platform other than the host is excluded, so using it
// is an error. The outcome depends on the host, so each host has its own
// copy gated on the other platform (see `// host:` in annotation.rs).

module Test

@platform(.linux)
struct ExcludedStruct {
    var x: lang.i64
}

func main() {
    let s = ExcludedStruct(x: 1); // ERROR: ExcludedStruct
}
