// Bytes moving through something: the stream protocols, their adapters,
// in-memory and buffered streams, standard input and output, and IoError.
//
// std.streams is the bottom I/O layer. std.files, std.network and
// std.system build on it; it depends on none of them.
//
// # Examples
//
// ```
// var input = Cursor(text: "one\ntwo\n").buffered();
// while let .Some(line) = try input.readLine() {
//     try println(line);
// }
// ```

module std.streams
