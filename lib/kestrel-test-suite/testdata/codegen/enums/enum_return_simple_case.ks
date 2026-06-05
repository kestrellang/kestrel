// test: execution
// stdlib: true

module Test

enum Direction {
    case Up
    case Down
    case Left
    case Right
}

func get_direction() -> Direction {
    Direction.Up
}

@main
func main() -> lang.i64 {
    let d = get_direction();
    match d {
        .Up => 0,
        _ => 1
    }
}
