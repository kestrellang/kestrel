// test: diagnostics

module Test

func generic[T](a: T, b: T) -> Bool where T: Equatable {
    a.notEqual(to: b) // ERROR: 'notEqual' can only be used through its syntax
}

func hooks() {
    var a = 7;
    a.remainderAssign(3); // ERROR: 'remainderAssign' can only be used through its syntax
    let _ = a.equal(to: 3); // ERROR: 'equal' can only be used through its syntax
    let _ = a.lessThan(3); // ERROR: 'lessThan' can only be used through its syntax
    let _ = a.exclusiveRange(to: 3); // ERROR: 'exclusiveRange' can only be used through its syntax
    let _ = a.bitwiseAnd(1); // ERROR: 'bitwiseAnd' can only be used through its syntax
    let o: Int64? = .Some(1);
    let _ = o.tryExtract(); // ERROR: 'tryExtract' can only be used through its syntax
    let _ = o.coalesce(2); // ERROR: 'coalesce' can only be used through its syntax
    let _ = [1, 2].matchGet(index: 0); // ERROR: 'matchGet' can only be used through its syntax
    let _ = Int64(intLiteral: 5); // ERROR: 'init(intLiteral:)' can only be used through its syntax
}

// Hooks that read well as methods stay public.
func visible() -> Bool {
    let a = 7;
    a.add(1).isEqual(to: 8)
}
