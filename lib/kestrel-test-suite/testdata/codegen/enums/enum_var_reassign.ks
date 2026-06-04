// test: execution
// stdlib: true

module Test

enum State {
    case Off
    case On
}

func get_value(s: State) -> std.numeric.Int64 {
    match s {
        .Off => 0,
        .On => 42
    }
}

@main
func main() -> lang.i64 {
    var state = State.Off;
    if get_value(state) != 0 { return 1 }

    state = State.On;
    if get_value(state) != 42 { return 2 }

    0
}
