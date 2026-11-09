// test: diagnostics
// stdlib: true

// A failed compound assignment names the receiver's type. The receiver was
// poisoned before the message was built, so it read `_ !: AddAssign`.
module Test

func a() {
    var s = "a";
    s += "b"; // ERROR(E107): String !: AddAssign
}
