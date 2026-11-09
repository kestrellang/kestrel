// Rosetta Code: Command-line arguments
// https://rosettacode.org/wiki/Command-line_arguments
module Main

import std.system.Process

@main
func main() {
    let args = Process.arguments();
    println("program name given: \(args.count > 0)");
    println("\(args.count - 1) argument(s)");
    for (i, arg) in args.iter().enumerate().skip(1) {
        println("arg \(i): \(arg)");
    }
}
