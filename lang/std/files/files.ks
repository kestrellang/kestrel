// Paths, files, directories and metadata.
//
// std.files is built on std.streams: a `File` is `Readable`, `Writable`
// and `Seekable`. Calls that need a target-specific struct (`stat`,
// `dirent`) go through `files_shims.c`.
//
// # Examples
//
// ```
// let text = try readText("notes.txt");
// try writeText("copy.txt", text);
// for entry in try Directory.entries(".") {
//     println((try entry).name);
// }
// ```

module std.files
