# Deinitializers

A deinitializer runs when an instance is about to be freed. Use it to release resources the type holds — file handles, locks, network connections, foreign memory.

```swift
struct FileHandle {
    let fd: Int

    deinit {
        close_fd(self.fd)
    }
}
```

Kestrel uses **automatic reference counting** to manage memory. When the last reference to an instance goes away, the runtime calls `deinit` (if you defined one) and then frees the memory. You don't call `deinit` yourself, and you don't pick the moment — the runtime decides based on reference counts.

For most struct types you'll never need a deinitializer. Reach for one only when the type owns something that must be cleaned up explicitly. See [Concepts → Memory Model](../concepts/memory-model.md) for how ARC decides when to run it.

A deinitializer takes no parameters and has no return value. It can read fields freely but should not, for example, store `self` somewhere — by the time `deinit` runs, the object is on its way out.

---

[← Initializers](initializers.md) · [↑ Structs](index.md) · [Computed Variables →](computed-variables.md)
