// Memory layout types

module std.memory

import std.core.(Equatable, Bool)
import std.numeric.(Int64)

/// Size + alignment pair describing the memory footprint of a type.
///
/// Allocators take a `Layout` rather than a raw byte count so they can
/// honour alignment requirements (SIMD types, page-aligned buffers, etc.).
/// The static `of[T]` and `array[T]` factories cover the common cases;
/// `merge` and `padToAlign` exist for hand-rolled struct layouts.
///
/// # Examples
///
/// ```
/// let l = Layout.of[Int64]();           // size 8, alignment 8
/// let buf = Layout.array[UInt8](1024);  // size 1024, alignment 1
/// allocator.allocate(l)
/// ```
///
/// # Representation
///
/// Two `Int64`s — `size` and `alignment`. No invariants enforced at
/// construction; misaligned layouts are caught (or undefined) at the
/// allocator level.
public struct Layout: Equatable {
    /// Footprint in bytes.
    public var size: Int64
    /// Required alignment in bytes — always a power of two for layouts
    /// produced by `of`/`array`.
    public var alignment: Int64

    /// @name From Fields
    /// Builds a layout from explicit `size` and `alignment`. Caller is
    /// responsible for keeping `alignment` a power of two.
    public init(size size: Int64, alignment alignment: Int64) {
        self.size = size;
        self.alignment = alignment;
    }

    /// Layout for a single value of `T` — uses the compiler-known
    /// `sizeof` and `alignof` for the type.
    public static func of[T]() -> Layout where T: not Copyable {
        Layout(size: Int64(intLiteral: lang.sizeof[T]()), alignment: Int64(intLiteral: lang.alignof[T]()))
    }

    /// Layout for `count` contiguous `T` values. Inherits the element's
    /// alignment; size is `sizeof[T] * count` with no inter-element padding
    /// (T is assumed already padded to its own alignment).
    public static func array[T](count: Int64) -> Layout {
        let elementLayout = Layout.of[T]();
        Layout(
            size: elementLayout.size * count,
            alignment: elementLayout.alignment
        )
    }

    /// Equal when both fields match.
    public func isEqual(to other: Layout) -> Bool {
        self.size == other.size and self.alignment == other.alignment
    }

    /// Rounds `size` up to the next multiple of `alignment`. Use when
    /// emitting a value into a packed array — without padding, element
    /// `i+1` would land at the wrong offset.
    public func padToAlign() -> Layout {
        let padding = (self.alignment - (self.size % self.alignment)) % self.alignment;
        Layout(size: self.size + padding, alignment: self.alignment)
    }

    /// Concatenates `other` after `self`, mimicking how a C struct lays
    /// out its second field. Returns the combined layout and the byte
    /// offset where `other`'s storage starts (handy for building field
    /// access tables by hand).
    public func merge(with other: Layout) -> (Layout, Int64) {
        let newAlign = if self.alignment > other.alignment {
            self.alignment
        } else {
            other.alignment
        };
        let padding = (other.alignment - (self.size % other.alignment)) % other.alignment;
        let offset = self.size + padding;
        let newSize = offset + other.size;
        (Layout(size: newSize, alignment: newAlign), offset)
    }

    // Repeat layout for array
    // Note: Requires Optional which comes in Phase 11
    // public func repeat(count: Int64) -> Optional[Layout] {
    //     if count == 0 {
    //         return .Some(Layout(size: 0, alignment: self.alignment))
    //     }
    //
    //     let padded = self.padToAlign();
    //     .Some(Layout(
    //         size: padded.size * count,
    //         alignment: self.alignment
    //     ))
    // }
}
