# Writing a Language in 6 months

## Why another language?

Writing a compiler has been a goal of mine for a long time. It's been my everest, not only because it is one of the hardest mountains in tech to climb, but also because it lets me own more of the stack. I can use my own tooling and understand how everything works from top to bottom. I've had a number of failed projects along the way, or as I like to call them learning experiences. Through this, and through working in many other languages, I was able to understand exactly what I wanted in a programming language. I wanted a language with elegant syntax, static typing, native compilation, precision in what it allowed, but safe defaults for quick prototyping. In addition, I was interested in bringing niche research programming language features into the mainstream with easy to understand syntax.

## The learning experiences

Over the years, I've tried a few compiler projects, and learned a lot along the way. It was difficult to architect around all the features I wanted. Type inference, generics, protocols, type aliases. It always broke somewhere along the way. Eventually, I was able to get a few languages to the point where they could compile. But working on them became a nightmare. They were minimally tested, logic was strewn around the codebase, and adding features was a huge challenge. I learned a lot about what features were important to me in a language, and how to structure it.

## What clicked

During the attempt before Kestrel, called Firefly, I realized something. If I horizontally sliced features and consolidated their logic, I wouldn't have to write method overloading code for structs and enums and in conforming methods for both. I wouldn't have to write type lookup code for anything that could be a type. I wouldn't have to have visibility logic everywhere.aI begun to organize the codebase into components, which described behaviors. I built something which sort of resembled the rust compiler's query system in computed components, which were derived from raw components and the context each node was in. The idea was good, but the execution was lacking.

After a few years I was bored and decided to revisit the concept. I started playing around with the project using Claude Code to test if it could work in a non-trivial codebase. To my surprise, it worked very well. It could build features end to end, and change logic without completely breaking everything. I decided the whole language idea merited another shot, and got to work brainstorming improvements to the architecture. After graduating and working professionally in tech for a few years, I knew how to execute on the concept much much better.

Realizing this idea was similar to the ECS architecture, I decided to base it on existing architectural concepts rather than do my own thing. I created a variant of ECS which I call hECS, or hierarchical entity component system. This system views the AST as a hierarchical tree of entities, each with a list of components describing how they should behave. Instead of manually calling computed components, I borrowed from Rust's query system to create a lazy loading, dependency tracked way of computing what I needed. And I borrowed Roslyn's analyzer system. And most importantly, I would write tests for every feature, every edge case, every error.

I built out the bones of the compiler, and put it aside for a month. I revisited it, and started building. In a weekend, I was able to build out the type system, structs, functions, function overloading, protocols, generics, protocol conformance, inheritance. It felt amazing to be able to build the thing that had stumped me for so long, so fast. Over the next week, I built out the statement and expression semantics. I was finally up to type inference. I wanted to clean up the architecture for this, and took some time to do this. I built out type inference, closures, enums, pattern matching. I finally felt like I was ready to move onto generating code. I build out a MIR representation, lowering constructs to a lower level representation. I worked out the memory model, and started on actual codegen. Unfortunately when this was done, there were a lot of bugs to fix, but I fixed them, and got a real program running!

I then realized this was getting close to being a real language. I built out a standard library, and a lot of syntactic sugar, and built a webserver in a language I had written! I started thinking about shipping it. I wanted to be able to provide a LSP for IDE integration, and started looking into how to do that. Unfortunately, after some trying, I realized the codebase was quite bloated and wouldn't allow this, and that I would have to start over again.

With my extensive test suite, this ended up being a much smaller task than I thought. I rewrote it, making sure to take into account all the features I had added along the way. I designed every system much more thoroughly. And eventually got through it. Of course, once I got there, 100s of tests were failing, and I had to add a lot of features that I missed, and a lot of error messages. But once I got the first program running, I was quickly able to get my webserver going again.

I used the new compiler to build a LSP. I think the biggest moment when it felt real was when I typed out a hello world program in the IDE. After selecting print from the autocomplete menu, the overloads of print in the standard library popped up in a dialog box, just like in any other language. I realized this attempt would be the one where I got over the hump.

## What is Kestrel?

Kestrel looks like it has training wheels but doesn't - every convenience is a thin wrapper over machinery you can see, touch, and replace. Kestrel is easy to read when you use the convenient defaults and when you don't.

```
func findUser(id: Int64) -> User? {
    let user = try users.get(id);
    if user.age < 18 { return null };
    user
}

func greet(id: Int64) -> String {
    let user = findUser(id) ?? User(name: "guest", age: 0);
    "Hello, \{user.name}!"
}

func main() {
    for name in ["Alice", "Bob", "Carol"] {
        print(name)
    }
}
```

Nothing here is magic. `try`, `null`, `??`, `for-in`, `"\{}"`, `[]`, even `<` — every piece of syntax calls a protocol method defined in the standard library. And because they're protocols, you can conform your own types to them:

```
// Your type works with `try`
struct Fetch[T]: Tryable {
    // ...
    func tryExtract() -> ControlFlow[T, FetchError] { ... }
}

let data = try fetch("/api/users");

// Your type works with `for-in`
struct TreeWalker[T]: Iterator {
    type Item = T
    mutating func next() -> T? { ... }
}

for node in tree { ... }

// Your type works with `[a, b, c]`
struct Vec3: ExpressibleByArrayLiteral {
    type Element = Float64
    init(arrayLiteral: LiteralSlice[Float64]) { ... }
}

let direction: Vec3 = [0.0, 1.0, 0.0];

// Your type works with `??`
struct Cached[T]: Coalesce[T] {
    type Output = T
    func coalesce(default: () -> T) -> T { ... }
}

let value = cache.get("key") ?? computeExpensive();
```

The standard library uses the same protocols to implement `Optional`, `Result`, `Array`, and `String`. Your types get the same syntax because it's the same machinery.

## Where its going

Kestrel hasn't yet strayed too far off the beaten path for language design. Yet. There are a few features every language has that are still remaining to be implemented. Classes, Async/Await, Generators, Metaprogramming. And a few I'm very excited to get out. Instead of showing you the features, I'll show how they'll be used.

### Business Logic
```
struct Money {
  // Use code contracts to throw a compiler error
  // if an invariant isn't checked
  @ensures(value >= 0)
  var value: Float64;
}

struct Account {
  var balance: Money;

  mutating func withdraw(amount: Money) -> Money {
    self.balance -= amount.value; // error: self.balance must stay above 0

    return amount;
  }
}
```

### Embedded / High Performance

```
func performExpensiveOperation() {
  given std.allocator = ArenaAllocator();

  for i in 0..<100000 {
    performExpensiveOperation(i);
  }
}
```

### Testing

```
func testHarness(f: () -> ()) {
  given std.filesystem = MockFilesystem(),
        std.random = DeterministicRandom(42);
        database = MockDatabase();

  f();
} 
```

### Databases

```
func totalIncome() -> Float64 {
  select(Orders)
    .join(OrderItems, on: OrderItems.orderId == Orders.id)
    .join(Products, on: OrderItems.productId == Products.id)
    .where(Orders.status == .Complete)
    .sum(OrderItems.quantity * Products.price)
}
```

### UI

```
@reactive
struct UserProfile: View {
  let id: Int64
  let user = query { await fetchUser(id: id) }

  func body() -> View throws async {
    let u = try user.get();
    Column { Avatar(url: u.avatar); Text(u.name) }
  }
}

// The parent decides what loading and failure look like
// If it doesn't, the compiler throws an error
guard {
  UserProfile(id: 1)
} catch {
  loading => Skeleton(),
  error(e) => ErrorBanner(e)
}
```

### Safety

```
linear struct DbTransaction {
  consuming func commit() { ... }
  consuming func rollback() { ... }
}

func transfer(from: Account, to: Account, amount: Money) throws {
  let txn = try db.begin();

  try txn.debit(from, amount);
  try txn.credit(to, amount);

  txn.commit()
  // forgetting to call commit() or rollback() is a compile error
  // the compiler forces you to finish what you started
}
```

### Streams

```
func livePrices(symbol: String) async yields Price throws InvalidPriceError {
  let socket = await connect(symbol)
  loop {
    let price = await socket.receive()
    if price < 0.0 { throw InvalidPriceError() }
    yield price
  }
}

async func main() {
  async for price in livePrices("AAPL") {
    print(price)
  }
}
```

### Metaprogramming

```
// Write a JSON serializer once
@attribute
func Codable(type: TypeInfo) {
  func encode(self: type) -> Value {
    var obj = Value.object();
    for field in type.fields {
      obj.set(field.name, field.value.encode());
    }
    obj
  }

  func decode(from value: Value) -> type throws DecodeError {
    type(
      for field in type.fields {
        field.name: try field.type.decode(from: value.get(field.name))
      }
    )
  }
}

// Use it everywhere
@Codable
struct User {
  let name: String
  let age: Int64
  let email: String
}

let json = user.encode()
let user = try User.decode(from: json)
```

### Theming

```
func app() -> View {
  given Theme = Theme.dark

  Column {
    Header("Dashboard")
    UserCard(user)      // gets theme implicitly
    Sidebar()           // no prop drilling
  }
}

// Any nested component can access it
func avatar(user: User) -> View {
  using Theme;
  let colors = Theme.colors;
  Circle(color: colors.accent) {
    Image(url: user.avatar)
  }
}
```

## Try it out

Kestrel is an early preview, absolutely not ready to be used in production. I'd love for people to try it out, give their opinions on different design decisions, and ideas for the future of this project. Also the flock package registry (Yes, written in kestrel) is looking a bit bare... Install the agent plugin or our install script to get started