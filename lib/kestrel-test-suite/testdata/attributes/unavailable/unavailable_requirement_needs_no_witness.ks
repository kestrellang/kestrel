// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

module Test

protocol Store {
    func fetch() -> Int64

    @unavailable("removed in 0.18")
    func legacyGet() -> Int64
}

// No 'legacyGet' witness: the requirement can never be called.
struct Memory: Store {
    func fetch() -> Int64 { 7 }
}

func read[S](s: S) -> Int64 where S: Store { s.fetch() }

@main
func main() -> Int64 {
    read(Memory()) - 7
}
