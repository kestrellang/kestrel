// Memory layout types

module std.memory

import std.core.(Equatable, Int, Bool)
import std.result.(Optional)

public struct Layout: Equatable {
    public var size: Int
    public var alignment: Int

    public init(size size: Int, alignment alignment: Int) {
        self.size = size;
        self.alignment = alignment;
    }

    public static func of[T]() -> Layout {
        Layout(size: lang.sizeof[T](), alignment: lang.alignof[T]())
    }

    public static func array[T](count: Int) -> Layout {
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
    public func merge(with other: Layout) -> (Layout, Int) {
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
    public func repeat(count: Int) -> Optional[Layout] {
        if count == 0 {
            return .Some(Layout(size: 0, alignment: self.alignment))
        }

        let padded = self.padToAlign();
        .Some(Layout(
            size: padded.size * count,
            alignment: self.alignment
        ))
    }
}
