# Hello, World

Your first Kestrel program. Three minutes start to finish.

## Create the project

```sh
flock new hello
cd hello
```

`flock new` scaffolds a project: `flock.toml` (the manifest), `src/main.ks`, and a `.gitignore`. Open `src/main.ks` in your editor — it already says hello.

```swift
module Main

func main() {
    println("Hello, world!")
}
```

## Run it

```sh
flock run
```

Output:

```
Hello, world!
```

`flock run` builds the project (if needed) and runs the resulting binary. The build output lands in `target/` — that's not interesting yet, but it's where the compiled binary lives.

## Edit, repeat

Change the message:

```swift
print("Hello, \(name())!")

func name() -> String {
    "Kestrel"
}
```

Save and `flock run` again. The compile is incremental — changes rebuild only what they affect.

## What's next

You've seen functions, return values, string interpolation, modules, and imports. The [Tour](../tour/index.md) builds on these to walk through the language end-to-end with three small programs.

If you'd rather jump straight to the reference, [Values & Variables](../values-and-variables.md) is the linear guide's first chapter.
