// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

func create_many() {
    let r00 = Resource(id: 0);
    let r01 = Resource(id: 1);
    let r02 = Resource(id: 2);
    let r03 = Resource(id: 3);
    let r04 = Resource(id: 4);
    let r05 = Resource(id: 5);
    let r06 = Resource(id: 6);
    let r07 = Resource(id: 7);
    let r08 = Resource(id: 8);
    let r09 = Resource(id: 9);
    let r10 = Resource(id: 10);
    let r11 = Resource(id: 11);
    let r12 = Resource(id: 12);
    let r13 = Resource(id: 13);
    let r14 = Resource(id: 14);
    let r15 = Resource(id: 15);
    let r16 = Resource(id: 16);
    let r17 = Resource(id: 17);
    let r18 = Resource(id: 18);
    let r19 = Resource(id: 19);
    let r20 = Resource(id: 20);
    let r21 = Resource(id: 21);
    let r22 = Resource(id: 22);
    let r23 = Resource(id: 23);
    let r24 = Resource(id: 24);
    let r25 = Resource(id: 25);
    let r26 = Resource(id: 26);
    let r27 = Resource(id: 27);
    let r28 = Resource(id: 28);
    let r29 = Resource(id: 29);
    let r30 = Resource(id: 30);
    let r31 = Resource(id: 31);
    let r32 = Resource(id: 32);
    let r33 = Resource(id: 33);
    let r34 = Resource(id: 34);
    let r35 = Resource(id: 35);
    let r36 = Resource(id: 36);
    let r37 = Resource(id: 37);
    let r38 = Resource(id: 38);
    let r39 = Resource(id: 39);
    let r40 = Resource(id: 40);
    let r41 = Resource(id: 41);
    let r42 = Resource(id: 42);
    let r43 = Resource(id: 43);
    let r44 = Resource(id: 44);
    let r45 = Resource(id: 45);
    let r46 = Resource(id: 46);
    let r47 = Resource(id: 47);
    let r48 = Resource(id: 48);
    let r49 = Resource(id: 49);
}

func main() -> lang.i64 {
    create_many();
    if deinit_count != 50 { return 1; }
    0
}
