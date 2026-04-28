// Kestrel I/O Library
//
// A simple I/O library built on libc. The umbrella module re-exports the
// types and functions from the submodules below so users can `import std.io`
// and get readers, writers, files, and standard streams in one go.
//
// # Examples
//
// ```
// import std.io
//
// try io.print("Hello, ");
// try io.println("World!");
//
// var file = try io.File.open("hello.txt");
//
// let name = try io.prompt("Name: ");
// try io.println("Hello, " + name);
// ```

module std.io

// ============================================================================
// RE-EXPORTS FROM SUBMODULES
// ============================================================================

// Low-level libc bindings
import std.io.libc

// Error types
import std.io.error.(IoError)

// Readable protocol and implementations
import std.io.read.(Readable, Empty, Repeat, Cursor, readByte, readAll)

// Writable protocol and implementations
import std.io.write.(Writable, Sink, Buffer, writeAll, writeByte, writeString, writeLine)

// File I/O
import std.io.file.(Seek, File, readFileString, writeFileString, appendFileString)

// Standard I/O streams
import std.io.stdio.(Stdin, Stdout, Stderr, stdin, stdout, stderr, print, println, printlnEmpty, eprint, eprintln, readLine, prompt)
