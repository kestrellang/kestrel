// The platform this program was compiled for.

module std.system

import std.numeric.(Int64)
import std.core.(Bool, Equatable)
import std.text.(String, Formatter, Formattable, FormatOptions)

/// An operating system Kestrel can target. Case names match the
/// `@platform(.Darwin)` / `@platform(.Linux)` attribute.
public enum OperatingSystem: Equatable, Formattable {
    /// macOS, iOS and the other Apple systems.
    case Darwin
    /// Linux, with any C library.
    case Linux
    /// WebAssembly in a browser, via Emscripten.
    case Emscripten

    public func isEqual(to other: OperatingSystem) -> Bool {
        match (self, other) {
            (.Darwin, .Darwin) => true,
            (.Linux, .Linux) => true,
            (.Emscripten, .Emscripten) => true,
            _ => false
        }
    }

    /// The lowercase name used in target triples: `darwin`, `linux`, `emscripten`.
    public var name: String {
        match self {
            .Darwin => "darwin",
            .Linux => "linux",
            .Emscripten => "emscripten"
        }
    }

    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.pad(self.name, options)
    }
}

/// A CPU architecture. Cases are added as Kestrel gains targets.
public enum Architecture: Equatable, Formattable {
    /// 64-bit x86 (`x86_64`, `amd64`).
    case X86_64
    /// 64-bit ARM (`aarch64`, Apple silicon).
    case Arm64
    /// 32-bit x86 (`i686`).
    case X86
    /// 32-bit ARM.
    case Arm32
    /// 64-bit RISC-V.
    case RiscV64
    /// 32-bit WebAssembly.
    case Wasm32

    public func isEqual(to other: Architecture) -> Bool {
        self.name.isEqual(to: other.name)
    }

    /// The name used in target triples and release asset names:
    /// `x86_64`, `arm64`, `x86`, `arm`, `riscv64`, `wasm32`.
    public var name: String {
        match self {
            .X86_64 => "x86_64",
            .Arm64 => "arm64",
            .X86 => "x86",
            .Arm32 => "arm",
            .RiscV64 => "riscv64",
            .Wasm32 => "wasm32"
        }
    }

    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.pad(self.name, options)
    }
}

/// Byte order of multi-byte integers in memory.
public enum Endianness: Equatable {
    case Little
    case Big

    public func isEqual(to other: Endianness) -> Bool {
        match (self, other) {
            (.Little, .Little) => true,
            (.Big, .Big) => true,
            _ => false
        }
    }
}

/// The target this program was compiled for: OS, CPU, pointer width and
/// byte order.
///
/// `Platform.current` describes the build target, not a guess made at run
/// time, so an x86_64 binary under Rosetta reports `X86_64`.
///
/// # Examples
///
/// ```
/// let p = Platform.current;
/// let asset = "tool-\(p.operatingSystem)-\(p.architecture).tar.gz";
/// ```
public struct Platform: Equatable, Formattable {
    public var operatingSystem: OperatingSystem
    public var architecture: Architecture
    /// Bits in a pointer: 32 or 64.
    public var pointerWidth: Int64
    public var endianness: Endianness

    /// The platform this program was compiled for.
    public static var current: Platform {
        Platform(
            operatingSystem: currentOperatingSystem(),
            architecture: currentArchitecture(),
            pointerWidth: Int64(from: unsafe native_pointer_width()),
            endianness: if unsafe native_is_little_endian() != 0 { .Little } else { .Big }
        )
    }

    public func isEqual(to other: Platform) -> Bool {
        self.operatingSystem.isEqual(to: other.operatingSystem)
            and self.architecture.isEqual(to: other.architecture)
            and self.pointerWidth == other.pointerWidth
            and self.endianness.isEqual(to: other.endianness)
    }

    /// Prints `<architecture>-<os>`, for example `arm64-darwin`.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.pad("\(self.architecture.name)-\(self.operatingSystem.name)", options)
    }
}

@platform(.Darwin)
internal func currentOperatingSystem() -> OperatingSystem { .Darwin }

@platform(.Linux)
internal func currentOperatingSystem() -> OperatingSystem { .Linux }

@platform(.Emscripten)
internal func currentOperatingSystem() -> OperatingSystem { .Emscripten }

/// The shim's answer comes from the C compiler's target macros, so it is
/// right for cross-compiled builds too. `@platform` can't select on
/// architecture yet; when it can, this becomes a compile-time constant.
internal func currentArchitecture() -> Architecture {
    match unsafe native_architecture() {
        0 => .X86_64,
        1 => .Arm64,
        2 => .X86,
        3 => .Arm32,
        4 => .RiscV64,
        _ => .Wasm32
    }
}
