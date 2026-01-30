// Kestrel I/O Library
//
// A simple I/O library built on libc.
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

// ============================================================================
// RE-EXPORTS FROM SUBMODULES
// ============================================================================

// Low-level libc bindings
import std.io.libc

// Error types
import std.io.error.(Error)

// Read protocol and implementations
import std.io.read.(Read, Empty, Repeat, Cursor, readByte, readAll)

// Write protocol and implementations
import std.io.write.(Write, Sink, Buffer, writeAll, writeByte, writeStr, writeLine)

// File I/O
import std.io.file.(Seek, File, readFileString, writeFileString, appendFileString)

// Standard I/O streams
import std.io.stdio.(Stdin, Stdout, Stderr, stdin, stdout, stderr, print, println, printlnEmpty, eprint, eprintln, readLine, prompt)
