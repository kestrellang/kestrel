// test: execution
// backends: cranelift,llvm
// stdlib: true

// A mutating METHOD called through a `&mutating` FIELD must mutate the
// pointee, not the slot. The address walk used to project FieldAddr of
// the slot itself — aliasing the stored pointer's bits as the receiver —
// and the write was silently lost (printed 5, not 6). The walk now stops
// at ref slots; the receiver routes through the loaded-ref view.
module Test

struct Counter {
    var n: Int64
    mutating func bump() { self.n = self.n + 1; }
}

struct Holder {
    var item: &mutating Counter
}

func pokeMut(mutating h: Holder) {
    h.item.bump();
}

func pokeShared(h: Holder) {
    h.item.bump();
}

@main
func main() -> lang.i64 {
    var c = Counter(n: 5);
    let r = &mutating c;
    var h = Holder(item: r);
    pokeMut(h);
    if c.n != 6 { return 1; }
    pokeShared(h);
    if c.n != 7 { return 2; }
    0
}
