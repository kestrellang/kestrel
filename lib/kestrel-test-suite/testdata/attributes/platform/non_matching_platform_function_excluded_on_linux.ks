// test: diagnostics
// stdlib: false
// host: linux

// A function gated on a platform other than the host is excluded, so using it
// is an error. The outcome depends on the host, so each host has its own
// copy gated on the other platform (see `// host:` in annotation.rs).

module Test

@platform(.Darwin)
func excluded() -> lang.i64 { 42 }

func main() {
    let x = excluded(); // ERROR: excluded
}
