// test: diagnostics
// stdlib: false

module Test

enum Status {
    case Pending
    case InProgress(percentage: lang.i64)
    case Completed
    case Failed(reason: lang.str)
}
