module datetime

import std.memory.(Pointer)
import std.ffi.(CString)

// IANA timezone backed by TZif data in a C-side global registry.
// Interned as a small integer ID — fully copyable.
// Reads /usr/share/zoneinfo/ for timezone data.
public struct TimeZone: Equatable, Hashable, Formattable {
    var id: Int64

    // --- Statics ---

    public static var utc: TimeZone { TimeZone.withId(0) }

    public static func system() -> TimeZone {
        var buf = Array[UInt8](repeating: 0, count: 256);
        kestrel_system_timezone_name(Pointer(to: buf(0)), 256);
        let name = String(from: CString(raw: Pointer(to: buf(0))));
        if let .Some(tz) = TimeZone.find(name) {
            return tz;
        }
        TimeZone.utc
    }

    public static func find(name: String) -> TimeZone? {
        let cstr = name.toCString();
        let tzId = kestrel_tz_find_or_register(cstr);
        cstr.free();
        if tzId < 0 { return .None; }
        .Some(TimeZone.withId(tzId))
    }

    static func withId(id: Int64) -> TimeZone {
        var tz = TimeZone.utc;
        tz.id = id;
        tz
    }

    // --- Properties ---

    public var name: String {
        var buf = Array[UInt8](repeating: 0, count: 128);
        kestrel_tz_name(self.id, Pointer(to: buf(0)), 128);
        String(from: CString(raw: Pointer(to: buf(0))))
    }

    // Get the UTC offset in seconds at a given instant
    func offsetAt(epochSec: Int64) -> Int64 {
        Int64(from: kestrel_tz_offset(self.id, epochSec))
    }

    // Get the timezone abbreviation at a given instant
    func abbreviationAt(epochSec: Int64) -> String {
        var buf = Array[UInt8](repeating: 0, count: 32);
        kestrel_tz_abbr(self.id, epochSec, Pointer(to: buf(0)), 32);
        String(from: CString(raw: Pointer(to: buf(0))))
    }

    // Whether a civil datetime is ambiguous (DST fold) in this timezone.
    // A fold occurs during fall-back when the same wall-clock time maps to
    // two different instants.
    public func isAmbiguous(dateTime dt: DateTime) -> Bool {
        let (naiveSecs, _) = dt.toEpochSecs();
        // First guess: use the offset at the naive epoch
        let off1 = self.offsetAt(naiveSecs);
        let epoch1 = naiveSecs - off1;
        let actualOff1 = self.offsetAt(epoch1);
        // Second guess: use the actual offset
        let epoch2 = naiveSecs - actualOff1;
        let actualOff2 = self.offsetAt(epoch2);
        // Ambiguous if two different instants both produce this civil time
        if actualOff1 == off1 { return false }
        // Verify both instants round-trip to the same civil time
        let civil1 = epoch1 + actualOff1;
        let civil2 = epoch2 + actualOff2;
        civil1 == naiveSecs and civil2 == naiveSecs and epoch1 != epoch2
    }

    // Whether a civil datetime is nonexistent (DST gap) in this timezone.
    // A gap occurs during spring-forward when a range of wall-clock times
    // are skipped entirely.
    public func isNonexistent(dateTime dt: DateTime) -> Bool {
        let (naiveSecs, _) = dt.toEpochSecs();
        // Convert to instant and back — if we don't get the same civil time, it's a gap
        let off = self.offsetAt(naiveSecs);
        let epochGuess = naiveSecs - off;
        let actualOff = self.offsetAt(epochGuess);
        let roundTrip = epochGuess + actualOff;
        roundTrip != naiveSecs
    }

    // --- Robust DST detection (commented out for future refinement) ---
    //
    // The above implementations handle common cases but may miss edge cases
    // near multiple rapid transitions (e.g., historical timezone changes).
    // A fully robust approach would scan the transition table directly:
    //
    // func findBracketingTransitions(dateTime dt: DateTime) -> (Int64, Int64)? {
    //     let (naiveSecs, _) = dt.toEpochSecs();
    //     let count = kestrel_tz_transition_count(self.id);
    //     var i: Int64 = 0;
    //     while i < count {
    //         var transEpoch: Int64 = 0;
    //         var offBefore: Int32 = 0;
    //         var offAfter: Int32 = 0;
    //         kestrel_tz_transition_at(self.id, i,
    //                                  Pointer(to: transEpoch),
    //                                  Pointer(to: offBefore),
    //                                  Pointer(to: offAfter));
    //         let civilBefore = transEpoch + Int64(from: offBefore);
    //         let civilAfter = transEpoch + Int64(from: offAfter);
    //         let offDiff = Int64(from: offAfter) - Int64(from: offBefore);
    //
    //         if offDiff > 0 {
    //             // Spring forward (gap): civil times in [civilBefore, civilAfter) don't exist
    //             if naiveSecs >= civilBefore and naiveSecs < civilAfter {
    //                 return .Some((Int64(from: offBefore), Int64(from: offAfter)));
    //             }
    //         } else if offDiff < 0 {
    //             // Fall back (fold): civil times in [civilAfter, civilBefore) are ambiguous
    //             if naiveSecs >= civilAfter and naiveSecs < civilBefore {
    //                 return .Some((Int64(from: offBefore), Int64(from: offAfter)));
    //             }
    //         }
    //         i = i + 1;
    //     }
    //     .None
    // }
    //
    // // More precise isAmbiguous using transition scanning:
    // func isAmbiguousPrecise(dateTime dt: DateTime) -> Bool {
    //     if let .Some((offBefore, offAfter)) = self.findBracketingTransitions(dt) {
    //         offAfter < offBefore  // fall-back = fold = ambiguous
    //     } else {
    //         false
    //     }
    // }
    //
    // // More precise isNonexistent using transition scanning:
    // func isNonexistentPrecise(dateTime dt: DateTime) -> Bool {
    //     if let .Some((offBefore, offAfter)) = self.findBracketingTransitions(dt) {
    //         offAfter > offBefore  // spring-forward = gap = nonexistent
    //     } else {
    //         false
    //     }
    // }

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
