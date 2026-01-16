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

module io

// Re-export from submodules
import io.libc
import io.error.(Error)
import io.read.(Read, Empty, Repeat, Cursor, readByte, readAll)
import io.write.(Write, Sink, Buffer, writeAll, writeByte, writeStr, writeLine)
import io.file.(Seek, File, readFileString, writeFileString, appendFileString)
import io.stdio.(Stdin, Stdout, Stderr, stdin, stdout, stderr, print, println, printlnEmpty, eprint, eprintln, readLine, prompt)
