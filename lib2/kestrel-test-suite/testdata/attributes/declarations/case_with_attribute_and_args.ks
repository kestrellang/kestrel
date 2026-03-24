// test: diagnostics
// stdlib: false

module Test
enum Priority {
    @dummy(level: 1)
    case High
    @dummy(level: 2)
    case Medium
    @dummy(level: 3)
    case Low
}
