// Mutating func on a non-generic struct that appends Entry[String].

module repro.main

struct Entry[T]: Cloneable {
    var name: String
    var items: Array[String]
    var handler: (String, T) -> String

    func clone() -> Entry[T] {
        Entry[T](
            name: self.name.clone(),
            items: self.items.clone(),
            handler: self.handler
        )
    }
}

struct Box: Cloneable {
    var entries: Array[Entry[String]]

    func clone() -> Box {
        Box(entries: self.entries.clone())
    }

    mutating func add(name: String, items: Array[String], handler: (String, String) -> String) {
        self.entries.append(Entry[String](
            name: name,
            items: items,
            handler: handler
        ))
    }
}

func main() {
    var b = Box(entries: Array[Entry[String]]());

    b.add("/", Array[String](), { (a: String, b: String) in "x" });

    var items2 = Array[String]();
    items2.append("x");
    b.add("/x", items2, { (a: String, b: String) in "y" });

    // Expected: 0, 1
    for entry in b.entries {
        println(entry.items.count.formatted());
    }
}
