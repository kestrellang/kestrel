// test: execution
// stdlib: true
// expect-stdout: HELLO\n

// A started child's stdin and stdout are pipes the parent writes and reads.

module Test

@main
func main() -> lang.i64 {
    var command = Command("tr");
    command.arguments(["a-z", "A-Z"]).standardInput(.Pipe).standardOutput(.Pipe);
    var child = match command.start() {
        .Ok(c) => c,
        .Err(_) => { return 1 }
    };
    match child.takeStandardInput() {
        .Some(input) => {
            var writer = input;
            let bytes: Array[UInt8] = [104, 101, 108, 108, 111, 10];
            match writeAll(writer, from: bytes.asUnsafeSlice()) {
                .Ok(_) => {},
                .Err(_) => { return 2 }
            }
            // Dropping the writer here sends end-of-file to tr.
        },
        .None => { return 3 }
    }
    if child.takeStandardInput().isSome() { return 4 }
    match child.waitForOutput() {
        .Ok(o) => {
            if not o.status.isSuccess { return 5 }
            print(o.standardOutputText() ?? "?");
        },
        .Err(_) => { return 6 }
    }
    0
}
