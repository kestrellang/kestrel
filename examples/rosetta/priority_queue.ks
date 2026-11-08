// Rosetta Code: Priority queue
// https://rosettacode.org/wiki/Priority_queue
module Main

import std.collections.Heap

struct Task {
    let priority: Int64;
    let name: String;
}

// Heap pops the smallest element first, so order tasks by priority.
extend Task: Comparable {
    func isEqual(to other: Task) -> Bool { self.priority == other.priority }
    func compare(other: Task) -> Ordering { self.priority.compare(other.priority) }
}

@main
func main() {
    var queue = Heap[Task]();
    queue.push(Task(priority: 3, name: "Clear drains"));
    queue.push(Task(priority: 4, name: "Feed cat"));
    queue.push(Task(priority: 5, name: "Make tea"));
    queue.push(Task(priority: 1, name: "Solve RC tasks"));
    queue.push(Task(priority: 2, name: "Tax return"));
    while let .Some(task) = queue.pop() {
        println("\(task.priority) \(task.name)");
    }
}
