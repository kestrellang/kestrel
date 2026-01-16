// Memory layout types

module std.memory

import std.core.(Equatable, Bool)
import std.num.(Int64)

public struct Layout: Equatable {
    public var size: Int64
    public var alignment: Int64

    public init(size size: Int64, alignment alignment: Int64) {
        self.size = size;
        self.alignment = alignment;
    }

    public static func of[T]() -> Layout {
        Layout(size: Int64(intLiteral: lang.sizeof[T]()), alignment: Int64(intLiteral: lang.alignof[T]()))
    }

    public static func array[T](count: Int64) -> Layout {
        let elementLayout = Layout.of[T]();
        Layout(
            size: elementLayout.size * count,
            alignment: elementLayout.alignment
        )
    }

    public func equals(other: Layout) -> Bool {
        self.size == other.size and self.alignment == other.alignment
    }

    // Pad size to alignment
    public func padToAlign() -> Layout {
        let padding = (self.alignment - (self.size % self.alignment)) % self.alignment;
        Layout(size: self.size + padding, alignment: self.alignment)
    }

    // Extend layout to include another layout (for struct field layout)
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
