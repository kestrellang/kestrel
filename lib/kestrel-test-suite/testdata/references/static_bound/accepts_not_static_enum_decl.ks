// test: execution
// backends: cranelift,llvm
// stdlib: true

// Enum form of the `: not Static` declaration (references 2a).

module Test

enum Slot: not Static {
    case Empty
    case Filled(Int64)
}

@main
func main() {
    let s = Slot.Filled(3);
    match s {
        .Filled(n) => {
            if n != 3 {
                fatalError("payload mismatch");
            }
        },
        .Empty => fatalError("wrong case"),
    }
    print("ok");
}

// CHECK: ok
