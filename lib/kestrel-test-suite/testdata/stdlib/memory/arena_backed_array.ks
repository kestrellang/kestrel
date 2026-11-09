// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 1000 499500 64 true 3 6 65536\n

// An Array can take an ArenaAllocator: growth reallocates in place inside the
// arena, clones keep the same allocator, and reset() reuses the memory.
module Test

import std.memory.(ArenaAllocator)

func firstRound(arena: ArenaAllocator) {
    var xs = Array[Int64, ArenaAllocator](allocator: arena);
    for i in 0..<1000 { xs.append(i); }
    var sum: Int64 = 0;
    for x in xs { sum = sum + x; }
    var big = Array[Int64, ArenaAllocator](capacity: 64, allocator: arena);
    let cap = big.capacity;
    big.append(1);
    var copy = xs.clone();
    copy.append(5);
    let distinct = copy.count == xs.count + 1;
    print("\(xs.count) \(sum) \(cap) \(distinct) ");
}

func secondRound(arena: ArenaAllocator) {
    var ys = Array[Int64, ArenaAllocator](allocator: arena);
    ys.append(1); ys.append(2); ys.append(3);
    var t: Int64 = 0;
    for y in ys { t = t + y; }
    print("\(ys.count) \(t) \(arena.reservedBytes)\n");
}

@main
func main() {
    var arena = ArenaAllocator();
    firstRound(arena);
    arena.reset();
    secondRound(arena);
    arena.release();
}
