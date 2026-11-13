// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// `@derive(Cloneable)` writes a `clone()` that copies every field or payload:
// `Cloneable` fields are cloned, `Copyable` ones copied. It works with private
// fields and a custom init, and a type parameter needs only `Copyable`, so
// `Box[Int64]` is `Cloneable` too. A clone owns its own storage: changing it
// leaves the original alone.

module Main

import std.numeric.(Int64)

@derive(Cloneable)
struct Task {
    let name: String
    private var args: Array[String]
    var tries: Int64

    init(name name: String) {
        self.name = name;
        self.args = ["-v"];
        self.tries = 0;
    }

    mutating func addArg(arg: String) { self.args.append(arg); }
    func argCount() -> Int64 { self.args.count }
}

@derive(Cloneable)
struct Box[T] {
    var value: T
}

@derive(Cloneable)
enum Job {
    case Idle
    case Run(task: Task, Int64)
}

func duplicate[T](x: T) -> T where T: Cloneable { x.clone() }

@main
func main() -> Int64 {
    var original = Task(name: "build");
    var copy = original.clone();
    copy.addArg("--release");
    copy.tries = 3;
    if original.argCount() != 1 or original.tries != 0 { return 1 }
    if copy.argCount() != 2 or copy.tries != 3 { return 2 }
    original.addArg("-q");
    if copy.argCount() != 2 { return 3 }

    if duplicate(original).argCount() != 2 { return 4 }
    if duplicate(Box(value: 5)).value != 5 { return 5 }
    if duplicate(Box(value: "hi")).value != "hi" { return 6 }

    let job = Job.Run(task: copy, 7);
    let jobCopy = job.clone();
    match jobCopy {
        .Run(task: t, n) => { if t.argCount() != 2 or n != 7 { return 7 } },
        .Idle => { return 8 },
    }
    if let .Idle = Job.Idle.clone() {} else { return 9 }

    let tasks = [original, copy];
    let copies = tasks.iter().map { it.clone() }.collect();
    if copies.count != 2 or copies(1).tries != 3 { return 10 }
    0
}
