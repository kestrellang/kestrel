// test: execution
// stdlib: true
// Regression test: extension subscript setter on Array[CustomEnum]
// previously triggered a monomorphization error ("dispatch bug: call to
// 'Slice.subscript' has 3 type arg(s), function expects 2") because the
// witness dispatch for the Slice extension setter was missing.

module Test

enum Color {
    case Red
    case Green
    case Blue
}

func main() -> lang.i64 {
    var arr = Array[Color](repeating: Color.Red, count: 4);

    // Checked subscript set
    arr(0) = Color.Green;
    arr(2) = Color.Blue;

    // Unchecked subscript set
    arr(unchecked: 1) = Color.Blue;
    arr(unchecked: 3) = Color.Green;

    // Verify via unchecked read
    match arr(unchecked: 0) { .Green => {}, _ => return 1 }
    match arr(unchecked: 1) { .Blue => {}, _ => return 2 }
    match arr(unchecked: 2) { .Blue => {}, _ => return 3 }
    match arr(unchecked: 3) { .Green => {}, _ => return 4 }

    // Verify via checked read
    match arr(0) { .Green => {}, _ => return 5 }
    match arr(1) { .Blue => {}, _ => return 6 }

    0
}
