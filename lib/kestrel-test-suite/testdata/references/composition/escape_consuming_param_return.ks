// test: diagnostics
// stdlib: true

// References 2b: a ref-bearing aggregate rooted at a consuming receiver
// is destroyed with the call — the carrier variant of E496. (A borrow of
// a COPYABLE FIELD of self copies out first and roots at the temp — that
// shape is E494; borrowing self itself pins the param root.)
module Test

struct Box {
    var v: Int64

    consuming func wrapped() -> Optional[&Box] {
        let r = &self;
        let o: Optional[&Box] = .Some(r);
        o // ERROR(E496)
    }
}
