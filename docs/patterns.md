# Protocol Method Calls

When desugaring syntax that requires protocol conformance (e.g., `for` loops requiring `Iterable`):

1. Look up the protocol method's `SymbolId` via `BuiltinRegistry::method()`
2. Create a `MethodRef` with that `SymbolId` as the candidate
3. Wrap in a `Call` expression

This produces proper "does not conform to X" errors instead of "no member Y", and prevents normal methods with that name from coming through.