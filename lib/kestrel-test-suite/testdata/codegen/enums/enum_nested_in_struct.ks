// test: execution
// stdlib: true

module Test

enum Status {
    case Active
    case Inactive
    case Pending(reason: std.numeric.Int64)
}

struct Task {
    let id: std.numeric.Int64
    let status: Status
}

@main
func main() -> lang.i64 {
    let task = Task(id: 42, status: Status.Active);
    match task.status {
        .Active => {
            if task.id != 42 { return 1 }
            0
        },
        _ => 2
    }
}
