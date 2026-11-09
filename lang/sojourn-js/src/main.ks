module js

// The `sojourn-js` command:
//
//   sojourn-js parse [--module] FILE      print FILE's syntax tree
//   sojourn-js test262 DIR [--verbose]    run tc39/test262-parser-tests in DIR

import std.system.(Process)
import std.files.(File, Path, Directory)

@main
func main() -> Int64 {
    let args = Process.arguments();
    if args.count >= 3 and args(1) == "parse" {
        return runParse(args);
    }
    if args.count >= 3 and args(1) == "test262" {
        let verbose = args.count >= 4 and args(3) == "--verbose";
        return runTest262(args(2), verbose);
    }
    print("usage: sojourn-js parse [--module] FILE\n       sojourn-js test262 DIR [--verbose]\n");
    2
}

func runParse(args: Array[String]) -> Int64 {
    let isModule = args.count >= 4 and args(2) == "--module";
    let path = args(args.count - 1);
    let source = match File.readText(Path(path)) {
        .Ok(s) => s,
        .Err(_) => {
            print("can't read \(path)\n");
            return 1;
        }
    };
    let result = if isModule { parseModule(source) } else { parseScript(source) };
    match result {
        .Ok(program) => {
            print(dumpProgram(program));
            print("\n");
            0
        },
        .Err(e) => {
            let kind = if e.early { "early error" } else { "syntax error" };
            print("\(path):\(describePosition(source, e.position)): \(kind): \(e.message)\n");
            1
        }
    }
}

/// `line:column` (1-based) of code point offset `position`.
func describePosition(source: String, position: Int64) -> String {
    var line = 1;
    var column = 1;
    var i: Int64 = 0;
    for c in source.chars {
        if i >= position { break; }
        if c.value() == 10 {
            line = line + 1;
            column = 1;
        } else {
            column = column + 1;
        }
        i = i + 1;
    }
    "\(line):\(column)"
}

/// The files in `dir`, sorted by name.
func listFiles(dir: String) -> Array[String] {
    var names: [String] = [];
    match Directory.entries(Path(dir)) {
        .Ok(entries) => {
            for entry in entries {
                match entry {
                    .Ok(e) => {
                        if e.name.ends(with: ".js") { names.append(e.name); }
                    },
                    .Err(_) => {}
                }
            }
        },
        .Err(_) => {}
    }
    names.sort();
    names
}

func readFile(path: String) -> Optional[String] {
    match File.readText(Path(path)) {
        .Ok(s) => .Some(s),
        .Err(_) => .None
    }
}

func parseFile(name: String, source: String) -> Result[Program, SyntaxError] {
    if name.contains(".module.") { parseModule(source) } else { parseScript(source) }
}

/// Runs the four test262-parser-tests folders and prints a pass rate for
/// each. With `verbose`, lists every failing file and why.
func runTest262(dir: String, verbose: Bool) -> Int64 {
    var failures = 0;

    // pass/: must parse, and match the pass-explicit/ twin.
    var passOk = 0;
    var passTotal = 0;
    var explicitOk = 0;
    for name in listFiles("\(dir)/pass") {
        passTotal = passTotal + 1;
        let source = readFile("\(dir)/pass/\(name)") ?? "";
        match parseFile(name, source) {
            .Ok(program) => {
                passOk = passOk + 1;
                let explicitSource = readFile("\(dir)/pass-explicit/\(name)") ?? "";
                match parseFile(name, explicitSource) {
                    .Ok(explicit) => {
                        if dumpProgram(program) == dumpProgram(explicit) {
                            explicitOk = explicitOk + 1;
                        } else if verbose {
                            print("pass-explicit DIFFERENT \(name)\n");
                        }
                    },
                    .Err(e) => if verbose { print("pass-explicit ERROR \(name): \(e.message) at \(describePosition(explicitSource, e.position))\n"); }
                }
            },
            .Err(e) => if verbose { print("pass ERROR \(name): \(e.message) at \(describePosition(source, e.position))\n"); }
        }
    }

    // fail/: must be a syntax error.
    var failOk = 0;
    var failTotal = 0;
    for name in listFiles("\(dir)/fail") {
        failTotal = failTotal + 1;
        let source = readFile("\(dir)/fail/\(name)") ?? "";
        match parseFile(name, source) {
            .Ok(_) => if verbose { print("fail ACCEPTED \(name)\n"); },
            .Err(_) => failOk = failOk + 1
        }
    }

    // early/: must be rejected (ideally as an early error).
    var earlyOk = 0;
    var earlyTotal = 0;
    for name in listFiles("\(dir)/early") {
        earlyTotal = earlyTotal + 1;
        let source = readFile("\(dir)/early/\(name)") ?? "";
        match parseFile(name, source) {
            .Ok(_) => if verbose { print("early ACCEPTED \(name)\n"); },
            .Err(_) => earlyOk = earlyOk + 1
        }
    }

    failures = (passTotal - passOk) + (passTotal - explicitOk) + (failTotal - failOk) + (earlyTotal - earlyOk);
    print("pass:          \(passOk)/\(passTotal) parse\n");
    print("pass-explicit: \(explicitOk)/\(passTotal) same tree as pass/\n");
    print("fail:          \(failOk)/\(failTotal) rejected\n");
    print("early:         \(earlyOk)/\(earlyTotal) rejected\n");
    if failures == 0 { 0 } else { 1 }
}
