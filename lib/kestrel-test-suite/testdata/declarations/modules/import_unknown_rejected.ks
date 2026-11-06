// test: diagnostics

// An import must name a module, or a member of a module that exists.
// Unknown imports used to be dropped silently.

module Test

import std.nope // ERROR(E629): cannot find 'std.nope'
import std.numeric.NopeType // ERROR(E629): cannot find 'std.numeric.NopeType'
import std.text.(String, NotThere) // ERROR(E629): module 'std.text' has no member 'NotThere'
