module Aerie

import deflate.(Zlib)
import std.files.(Path, File)

/// The world's save file: a `SaveState`, zlib-compressed.
struct SaveFile {
    var path: String

    /// The save, if there is a readable one. A missing file is a new
    /// world; a damaged one is reported and ignored rather than
    /// overwritten silently on the next save.
    func load() -> SaveState? {
        guard let .Ok(packed) = File.readBytes(Path(self.path)) else {
            return .None;
        }
        guard let .Ok(bytes) = Zlib.decompress(packed) else {
            println("\(self.path) is not a readable save; starting a new world");
            return .None;
        }
        let state = SaveState.decode(bytes);
        if state.isNone {
            println("\(self.path) is from another version of Aerie; starting a new world");
        }
        state
    }

    /// Compress and write `state`, reporting how it went. True if it was
    /// written.
    func write(state: SaveState) -> Bool {
        let packed = Zlib.compress(state.encode());
        guard let .Ok(_) = File.writeBytes(Path(self.path), packed.asSlice()) else {
            println("couldn't write \(self.path)");
            return false;
        }
        persistSaves();
        println("saved \(state.chunks.count) changed chunks to \(self.path) (\(packed.count) bytes)");
        true
    }
}
