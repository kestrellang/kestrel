// test: diagnostics
// stdlib: false

module Test

struct Level1 {
    enum Level2 {
        case Value
        enum Level3 {
            case DeepValue
        }
    }
}
