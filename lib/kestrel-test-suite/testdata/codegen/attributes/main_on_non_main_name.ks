// test: execution
// expect-exit: 7

// The entry point is whatever free function carries `@main`, regardless of its
// name — discovery is no longer by the name `main`. Here the entry is named
// `entryPoint` and returns exit code 7 (proving codegen exports it as C `main`).

module Main

@main
func entryPoint() -> lang.i64 { 7 }
