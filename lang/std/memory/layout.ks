// Memory layout types

module std.memory

import std.core.(Equatable, Bool, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)

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
/// Two `Int64`s — `size` and `alignment`. Explicitly constructed layouts
/// are validated by allocators; the factories in this type reject negative
/// sizes, invalid alignments, and arithmetic overflow.
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

    /// Layout for `count` contiguous `T` values. Traps if `count` is negative
    /// or the total size cannot be represented by `Int64`. Use
    /// `arrayChecked` to handle those cases explicitly.
    public static func array[T](count: Int64) -> Layout where T: not Copyable {
        match Layout.arrayChecked[T](count) {
            .Some(layout) => layout,
            .None => fatalError("Layout.array: negative count or size overflow")
        }
    }

    /// Checked counterpart to `array`. Returns `None` for a negative count
    /// or when `sizeof[T] * count` overflows.
    public static func arrayChecked[T](count: Int64) -> Layout? where T: not Copyable {
        if count < 0 {
            return .None
        };
        let elementLayout = Layout.of[T]();
        match elementLayout.size.multiplyChecked(count) {
            .Some(size) => .Some(Layout(size: size, alignment: elementLayout.alignment)),
            .None => .None
        }
    }

    /// Equal when both fields match.
    public func isEqual(to other: Layout) -> Bool {
        self.size == other.size and self.alignment == other.alignment
    }

    /// Rounds `size` up to the next multiple of `alignment`. Traps for an
    /// invalid layout or arithmetic overflow; use `padToAlignChecked` to
    /// handle failure explicitly.
    public func padToAlign() -> Layout {
        match self.padToAlignChecked() {
            .Some(layout) => layout,
            .None => fatalError("Layout.padToAlign: invalid layout or size overflow")
        }
    }

    /// Checked counterpart to `padToAlign`.
    public func padToAlignChecked() -> Layout? {
        if self.size < 0 or not self.alignment.isPowerOfTwo {
            return .None
        };
        let padding = (self.alignment - (self.size % self.alignment)) % self.alignment;
        match self.size.addChecked(padding) {
            .Some(size) => .Some(Layout(size: size, alignment: self.alignment)),
            .None => .None
        }
    }

    /// Concatenates `other` after `self`, mimicking how a C struct lays
    /// out its second field. Returns the combined layout and the byte
    /// offset where `other`'s storage starts (handy for building field
    /// access tables by hand).
    public func merge(with other: Layout) -> (Layout, Int64) {
        match self.mergeChecked(with: other) {
            .Some(merged) => merged,
            .None => fatalError("Layout.merge: invalid layout or size overflow")
        }
    }

    /// Checked counterpart to `merge`. Returns `None` if either input is
    /// invalid or if padding/size arithmetic overflows.
    public func mergeChecked(with other: Layout) -> (Layout, Int64)? {
        if self.size < 0 or other.size < 0 or
            not self.alignment.isPowerOfTwo or not other.alignment.isPowerOfTwo {
            return .None
        };
        let newAlign = if self.alignment > other.alignment {
            self.alignment
        } else {
            other.alignment
        };
        let padding = (other.alignment - (self.size % other.alignment)) % other.alignment;
        match self.size.addChecked(padding) {
            .Some(offset) => match offset.addChecked(other.size) {
                .Some(newSize) => .Some((Layout(size: newSize, alignment: newAlign), offset)),
                .None => .None
            },
            .None => .None
        }
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
