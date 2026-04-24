// test: diagnostics
// stdlib: false

module Main

enum Toggle {
    case On
    case Off
}

extend Toggle {
    func asBool() -> lang.i1 {
        match self {
            .On => true,
            .Off => false
        }
    }
}
