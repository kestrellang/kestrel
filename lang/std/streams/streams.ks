// Bytes moving through something: the stream protocols, their adapters,
// in-memory and buffered streams, and standard input and output.
//
// std.streams is the bottom I/O layer. std.system, std.filesystem and
// std.net build on it; it depends on none of them.
//
// # Transition
//
// std.streams replaces the stream half of std.io. Until std.io is removed,
// every top-level declaration here is `internal` rather than `public`:
// std modules are all auto-imported into user code, so a internal `Readable`,
// `Buffer` or `println` here would make those names ambiguous next to
// std.io's. Other std modules (std.system today) already use it. Members
// are `internal` too, since a public member can't mention an internal type.
// The switch is replacing `internal` with `public` throughout.
//
// `IoError` stays in std.io until then too, so both libraries report the
// same error type. It moves here at the switch, with the new
// `InvalidData`, `UnexpectedEnd` and `TimedOut` kinds; until then those
// cases report `InvalidInput`.
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
