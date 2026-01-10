// Iterator extension methods

module std.iter

extend Iterator {
    // Transform
    public func map[U](transform: (Item) -> U) -> MapIterator[Self, U] {
        MapIterator(inner: self, transform: transform)
    }

    public func filter(predicate: (Item) -> Bool) -> FilterIterator[Self] {
        FilterIterator(inner: self, predicate: predicate)
    }

    public func filterMap[U](transform: (Item) -> Optional[U]) -> FilterMapIterator[Self, U] {
        FilterMapIterator(inner: self, transform: transform)
    }

    public func flatMap[U, I](transform: (Item) -> I) -> FlatMapIterator[Self, I]
        where I: Iterable, I.Item = U
    {
        FlatMapIterator(inner: self, transform: transform, current: .None)
    }

    public func inspect(action: (Item) -> Void) -> InspectIterator[Self] {
        InspectIterator(inner: self, action: action)
    }

    // Take and skip
    public func take(count: Int) -> TakeIterator[Self] {
        TakeIterator(inner: self, remaining: count)
    }

    public func takeWhile(predicate: (Item) -> Bool) -> TakeWhileIterator[Self] {
        TakeWhileIterator(inner: self, predicate: predicate, done: false)
    }

    public func skip(count: Int) -> SkipIterator[Self] {
        SkipIterator(inner: self, remaining: count)
    }

    public func skipWhile(predicate: (Item) -> Bool) -> SkipWhileIterator[Self] {
        SkipWhileIterator(inner: self, predicate: predicate, done: false)
    }

    public func stepBy(step: Int) -> StepByIterator[Self] {
        StepByIterator(inner: self, step: step, first: true)
    }

    // Combine
    public func enumerate() -> EnumerateIterator[Self] {
        EnumerateIterator(inner: self, index: 0)
    }

    public func zip[Other](with other: Other) -> ZipIterator[Self, Other] where Other: Iterator {
        ZipIterator(first: self, second: other)
    }

    public func chain[Other](other: Other) -> ChainIterator[Self, Other] where Other: Iterator, Other.Item = Item
    {
        ChainIterator(first: self, second: other, firstDone: false)
    }

    public func cycle() -> CycleIterator[Self] where Self: Cloneable {
        CycleIterator(original: self, current: self.clone())
    }

    public func intersperse(separator: Item) -> IntersperseIterator[Self] where Item: Cloneable {
        IntersperseIterator(inner: self, separator: separator, needsSeparator: false)
    }

    // Peek
    public func peekable() -> PeekableIterator[Self] {
        PeekableIterator(inner: self, peeked: .None)
    }

    public func fuse() -> FuseIterator[Self] {
        FuseIterator(inner: self, done: false)
    }

    // Consuming operations
    public func fold[Acc](initial: Acc, combine: (Acc, Item) -> Acc) -> Acc {
        var acc = initial;
        while let item = self.next() {
            acc = combine(acc, item)
        }
        acc
    }

    public func reduce(combine: (Item, Item) -> Item) -> Optional[Item] {
        var result: Optional[Item] = self.next();
        while let item = self.next() {
            result = result.map { (acc) in combine(acc, item) }
        }
        result
    }

    public func collect[C]() -> C where C: Collectable, C.Item = Item {
        C(from: self)
    }

    public func count() -> Int {
        var n = 0;
        while self.next().isSome {
            n = n + 1
        }
        n
    }

    public func forEach(action: (Item) -> Void) {
        while let item = self.next() {
            action(item)
        }
    }

    public func any(predicate: (Item) -> Bool) -> Bool {
        while let item = self.next() {
            if predicate(item) {
                return true
            }
        }
        false
    }

    public func all(predicate: (Item) -> Bool) -> Bool {
        while let item = self.next() {
            if not predicate(item) {
                return false
            }
        }
        true
    }

    public func find(predicate: (Item) -> Bool) -> Optional[Item] {
        while let item = self.next() {
            if predicate(item) {
                return .Some(item)
            }
        }
        .None
    }

    public func position(predicate: (Item) -> Bool) -> Optional[Int] {
        var i = 0;
        while let item = self.next() {
            if predicate(item) {
                return .Some(i)
            }
            i = i + 1
        }
        .None
    }

    public func last() -> Optional[Item] {
        var result: Optional[Item] = .None;
        while let item = self.next() {
            result = .Some(item)
        }
        result
    }

    public func nth(n: Int) -> Optional[Item] {
        var remaining = n;
        while let item = self.next() {
            if remaining == 0 {
                return .Some(item)
            }
            remaining = remaining - 1
        }
        .None
    }

    public func min() -> Optional[Item] where Item: Comparable {
        self.reduce { (a, b) in if a < b { a } else { b } }
    }

    public func max() -> Optional[Item] where Item: Comparable {
        self.reduce { (a, b) in if a > b { a } else { b } }
    }

    public func minBy(compare: (Item, Item) -> Ordering) -> Optional[Item] {
        self.reduce { a, b in
            match compare(a, b) {
                .Greater => b,
                _ => a
            }
        }
    }

    public func maxBy(compare: (Item, Item) -> Ordering) -> Optional[Item] {
        self.reduce { a, b in
            match compare(a, b) {
                .Less => b,
                _ => a
            }
        }
    }

    public func sum() -> Item where Item: Addable[Item] + Numeric {
        self.fold(initial: Item.zero) { (acc, item) in acc + item }
    }

    public func product() -> Item where Item: Multipliable[Item] + Numeric {
        self.fold(initial: Item.one) { (acc, item) in acc * item }
    }

    // Try operations
    public func tryFold[Acc, E](
        initial: Acc,
        combine: (Acc, Item) -> Residual[Acc, E]
    ) -> Residual[Acc, E] {
        var acc = initial
        while let item = self.next() {
            match combine(acc, item) {
                .Output(newAcc) => acc = newAcc,
                .Early(e) => return .Early(e)
            }
        }
        .Output(acc)
    }

    public func tryForEach[E](action: (Item) -> Residual[(), E]) -> Residual[(), E] {
        while let item = self.next() {
            match action(item) {
                .Output(_) => {},
                .Early(e) => return .Early(e)
            }
        }
        .Output(())
    }

    // Partition
    public func partition(predicate: (Item) -> Bool) -> ([Item], [Item])
        where Item: Cloneable
    {
        var trueItems: [Item] = []
        var falseItems: [Item] = []
        while let item = self.next() {
            if predicate(item) {
                trueItems.append(item)
            } else {
                falseItems.append(item)
            }
        }
        (trueItems, falseItems)
    }

    // Compare iterators
    public func eq[Other](other: Other) -> Bool
        where Other: Iterator, Other.Item = Item, Item: Equatable
    {
        while true {
            match (self.next(), other.next()) {
                (.Some(a), .Some(b)) => {
                    if a != b { return false }
                },
                (.None, .None) => return true,
                _ => return false
            }
        }
    }

    public func cmp[Other](other: Other) -> Ordering
        where Other: Iterator, Other.Item = Item, Item: Comparable
    {
        while true {
            match (self.next(), other.next()) {
                (.Some(a), .Some(b)) => {
                    let ord = a.compare(b)
                    if ord != .Equal { return ord }
                },
                (.None, .None) => return .Equal,
                (.None, .Some(_)) => return .Less,
                (.Some(_), .None) => return .Greater
            }
        }
    }
}
