// test: diagnostics
// stdlib: false

module Test

struct Resource {
    let id: lang.i64

    deinit {
        // Cleanup would happen here
        // Without side effects, we can't verify it was called
    }
}

func main() -> lang.i64 {
    let r = Resource(id: 42);
    r.id
}
