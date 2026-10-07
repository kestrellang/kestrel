//! The crate's hash collections: Fx hashing, never std's per-process random
//! seed. The solver iterates several of these maps, and with a random seed
//! the iteration order — and with it which constraint fires first, which
//! expression claims a deduplicated diagnostic, and so the *set* of
//! diagnostics — changed from run to run of the same binary (bidi design §10).
//! Use these aliases, not `std::collections::{HashMap, HashSet}`.

pub type HashMap<K, V> = rustc_hash::FxHashMap<K, V>;
pub type HashSet<T> = rustc_hash::FxHashSet<T>;
