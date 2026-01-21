// Kestrel I/O Library
//
// A simple I/O library built on libc, using std2.
//
// Example usage:
//
//   import io
//
//   // Print to stdout
//   try io.print(s: "Hello, ")
//   try io.println(s: "World!")
//
//   // Read from file
//   var file = try io.File.open(path: "hello.txt")
//
//   // Standard I/O
//   let name = try io.prompt(message: "Name: ")
//   try io.println(s: "Hello, " + name)

module std.io

// Re-export from submodules
import std.io.libc
import std.io.error.(Error)
import std.io.read.(Read, Empty, Repeat, Cursor, readByte, readAll)
import std.io.write.(Write, Sink, Buffer, writeAll, writeByte, writeStr, writeLine)
import std.io.file.(Seek, File, readFileString, writeFileString, appendFileString)
import std.io.stdio.(Stdin, Stdout, Stderr, stdin, stdout, stderr, print, println, printlnEmpty, eprint, eprintln, readLine, prompt)
