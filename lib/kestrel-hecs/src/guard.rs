//! `OnDrop` — run a closure when a scope exits, on the normal path *and*
//! while unwinding.
//!
//! # Why this exists
//!
//! Several places in the compiler bracket a piece of work between a "push"
//! and a "pop" of some side state: the query framework's active-query stack
//! (cycle detection), `OPAQUE_RESOLVE_STACK` in `kestrel-mir-lower`,
//! `COMPUTING_COPY_SEMANTICS` / `COMPUTING_STATICNESS` in
//! `kestrel-semantics`. A hand-written pop after the work is skipped when
//! the work panics — and the compiler has **five hosts that catch panics
//! and keep going** (`CompilerDriver::infer_all`, the LSP compiler worker,
//! the test-suite harness, and the two codegen backends' per-function
//! `catch_unwind`). Leaked state there is not a lost cleanup, it is a
//! *wrong answer* on every later unit of work on that thread: a fabricated
//! "Query cycle detected", an opaque origin that resolves to `error()`
//! forever, a nominal that answers `Copyable` forever.
//!
//! One primitive covers all of them. A stack-specific RAII type cannot
//! reach into a `LocalKey` without re-entering `.with()` anyway, so the
//! useful shape is always "a closure that knows how to undo one push".
//!
//! ```
//! use kestrel_hecs::guard::OnDrop;
//!
//! let mut popped = false;
//! {
//!     let _guard = OnDrop::new(|| popped = true);
//!     // ... work that may panic ...
//! }
//! assert!(popped);
//! ```
//!
//! For state that should only be rolled back on the *abnormal* exit (a
//! tentative mark that the normal path legitimately keeps), gate the body
//! on `std::thread::panicking()`.

/// Runs `f` when dropped. See the module docs.
///
/// Bind it to a named local (`let _guard = ...`), never to `_` — `let _ =`
/// drops the value immediately and the cleanup runs at once.
pub struct OnDrop<F: FnMut()>(F);

impl<F: FnMut()> OnDrop<F> {
    pub fn new(f: F) -> Self {
        Self(f)
    }
}

impl<F: FnMut()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn runs_on_normal_exit() {
        let ran = Cell::new(false);
        {
            let _guard = OnDrop::new(|| ran.set(true));
            assert!(!ran.get(), "must not run before the scope ends");
        }
        assert!(ran.get());
    }

    #[test]
    fn runs_while_unwinding() {
        let ran = Cell::new(false);
        let r = catch_unwind(AssertUnwindSafe(|| {
            let _guard = OnDrop::new(|| ran.set(true));
            panic!("boom");
        }));
        assert!(r.is_err());
        assert!(ran.get(), "guard must run on the unwind path");
    }

    #[test]
    fn guards_unwind_in_reverse_order() {
        // The push/pop sites rely on LIFO: an inner frame's guard must pop
        // before an outer frame's does, or a stack-shaped undo corrupts.
        let order = Cell::new(String::new());
        let push = |c: char| {
            let mut s = order.take();
            s.push(c);
            order.set(s);
        };
        {
            let _outer = OnDrop::new(|| push('o'));
            let _inner = OnDrop::new(|| push('i'));
        }
        assert_eq!(order.take(), "io");
    }

    #[test]
    fn panicking_flag_distinguishes_the_two_paths() {
        let normal = Cell::new(0u32);
        let unwind = Cell::new(0u32);
        let bump = |n: &Cell<u32>, u: &Cell<u32>| {
            if std::thread::panicking() {
                u.set(u.get() + 1);
            } else {
                n.set(n.get() + 1);
            }
        };

        {
            let _g = OnDrop::new(|| bump(&normal, &unwind));
        }
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _g = OnDrop::new(|| bump(&normal, &unwind));
            panic!("boom");
        }));

        assert_eq!((normal.get(), unwind.get()), (1, 1));
    }
}
