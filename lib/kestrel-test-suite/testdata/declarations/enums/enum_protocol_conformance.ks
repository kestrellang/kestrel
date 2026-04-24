// test: diagnostics
// stdlib: false

module Test

protocol Named {
    func name() -> lang.str
}
enum State: Named {
    case Active
    case Inactive
    func name() -> lang.str { return "State"; }
}
