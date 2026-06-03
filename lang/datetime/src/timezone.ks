module datetime

import std.memory.(Pointer)
import std.ffi.(CString)

// IANA timezone backed by TZif data in a C-side global registry.
// Interned as a small integer ID — fully copyable.
// Reads /usr/share/zoneinfo/ for timezone data.
public struct TimeZone: Equatable, Hashable, Formattable {
    var id: Int64

    // Primitive constructor — the single point that builds a TimeZone from a
    // raw interned id. `utc` and `init(name:)` route through this so neither
    // recurses through the other (a prior `utc -> withId -> utc` cycle stack-
    // overflowed).
    init(rawId id: Int64) { self.id = id; }

    // --- Statics ---

    public static var utc: TimeZone { TimeZone(rawId: 0) }

    public static func system() -> TimeZone {
        var buf = Array[UInt8](repeating: 0, count: 256);
        // `buf.asPointer()` addresses the array's storage. `Pointer(to: buf(0))`
        // would take the address of a *temporary copy* of element 0 (the
        // subscript getter returns a value, not a place), so the C function's
        // write would overflow that 1-byte temporary → stack corruption.
        kestrel_system_timezone_name(buf.asPointer(), 256);
        let name = String(from: CString(raw: buf.asPointer()));
        if let .Some(tz) = TimeZone(name) {
            return tz;
        }
        TimeZone.utc
    }

    // Look up an IANA timezone by name (e.g. "America/New_York"). Fails with
    // `nil` if the name isn't a registrable zone.
    public init(name: String)? {
        let cstr = name.toCString();
        let tzId = kestrel_tz_find_or_register(cstr);
        cstr.free();
        if tzId < 0 { return null; }
        self.id = tzId;
    }

    // --- Properties ---

    public var name: String {
        var buf = Array[UInt8](repeating: 0, count: 128);
        kestrel_tz_name(self.id, buf.asPointer(), 128);
        String(from: CString(raw: buf.asPointer()))
    }

    // Get the UTC offset in seconds at a given instant
    func offsetAt(epochSec: Int64) -> Int64 {
        Int64(from: kestrel_tz_offset(self.id, epochSec))
    }

    // Get the timezone abbreviation at a given instant
    func abbreviationAt(epochSec: Int64) -> String {
        var buf = Array[UInt8](repeating: 0, count: 32);
        kestrel_tz_abbr(self.id, epochSec, buf.asPointer(), 32);
        String(from: CString(raw: buf.asPointer()))
    }

    // Find the DST transition whose gap/fold window contains `dt`, returning
    // the (offsetBefore, offsetAfter) pair in seconds, or `.None` if the wall
    // time sits in no transition window. Scans the IANA transition table
    // directly: an offset-guessing heuristic cannot detect a fold, because a
    // fold's first candidate instant is self-consistent (it round-trips to the
    // same civil time), so there is nothing for a round-trip check to catch.
    func findBracketingTransitions(dateTime dt: DateTime) -> (Int64, Int64)? {
        let (naiveSecs, _) = dt.toEpochSecs();
        let count = kestrel_tz_transition_count(self.id);
        var i: Int64 = 0;
        while i < count {
            var transEpoch: Int64 = 0;
            var offBefore: Int32 = 0;
            var offAfter: Int32 = 0;
            kestrel_tz_transition_at(self.id, i,
                                     Pointer(to: transEpoch),
                                     Pointer(to: offBefore),
                                     Pointer(to: offAfter));
            let before = Int64(from: offBefore);
            let after = Int64(from: offAfter);
            // Civil (wall-clock) time on each side of the transition instant.
            let civilBefore = transEpoch + before;
            let civilAfter = transEpoch + after;
            if after > before {
                // Spring forward (gap): wall times in [civilBefore, civilAfter) never occur.
                if naiveSecs >= civilBefore and naiveSecs < civilAfter {
                    return .Some((before, after));
                }
            } else if after < before {
                // Fall back (fold): wall times in [civilAfter, civilBefore) occur twice.
                if naiveSecs >= civilAfter and naiveSecs < civilBefore {
                    return .Some((before, after));
                }
            }
            i = i + 1;
        }
        .None
    }

    // Whether a civil datetime is ambiguous — a DST fall-back fold, where the
    // same wall-clock time maps to two different instants. Internal: the public
    // entry point is `DateTime.isAmbiguous(in:)`.
    func isAmbiguous(dateTime dt: DateTime) -> Bool {
        if let .Some((before, after)) = self.findBracketingTransitions(dateTime: dt) {
            after < before
        } else {
            false
        }
    }

    // Whether a civil datetime is nonexistent — a DST spring-forward gap, where
    // a range of wall-clock times is skipped entirely. Internal: the public
    // entry point is `DateTime.isNonexistent(in:)`.
    func isNonexistent(dateTime dt: DateTime) -> Bool {
        if let .Some((before, after)) = self.findBracketingTransitions(dateTime: dt) {
            after > before
        } else {
            false
        }
    }

    // --- Protocol conformances ---

    public func isEqual(to other: TimeZone) -> Bool {
        self.id == other.id
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.id.hash(into: hasher);
    }

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        writer.append(self.name);
    }
}
