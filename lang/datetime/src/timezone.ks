module datetime

import std.memory.(Pointer)
import std.ffi.(CString)

// IANA or fixed-offset timezone backed by a C-side global registry.
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

    // Look up a zone by IANA name ("America/New_York") or fixed offset
    // ("+05:30"). Fails with `nil` if the name isn't a registrable zone.
    public init(name: String)? {
        let tzId = TimeZone.registeredId(name);
        if tzId < 0 { return null; }
        self.id = tzId;
    }

    // A fixed-offset zone (no DST), named like "+05:30". The offset must be a
    // whole number of minutes within ±23:59; zero is `TimeZone.utc`.
    public init(offsetSeconds offset: Int64)? {
        if offset % 60 != 0 or offset.abs() >= 86400 { return null; }
        // appendOffset spells zero "Z", which is not a zone name
        if offset == 0 {
            self.id = TimeZone.utc.id;
            return;
        }
        var b = StringBuilder();
        appendOffset(into: b, offset);
        let tzId = TimeZone.registeredId(b.build());
        if tzId < 0 { return null; }
        self.id = tzId;
    }

    // Find or register `name` in the C-side registry; -1 if it can't be loaded.
    static func registeredId(name: String) -> Int64 {
        let cstr = name.toCString();
        let tzId = kestrel_tz_find_or_register(cstr);
        cstr.free();
        tzId
    }

    // --- Properties ---

    public var name: String {
        var buf = Array[UInt8](repeating: 0, count: 128);
        kestrel_tz_name(self.id, buf.asPointer(), 128);
        String(from: CString(raw: buf.asPointer()))
    }

    // --- Queries ---

    // UTC offset in effect at `instant`, in seconds east of UTC.
    public func offset(at instant: Instant) -> Int64 {
        self.offsetAt(instant.secs)
    }

    // Abbreviation in effect at `instant` ("EDT"), for display only.
    public func abbreviation(at instant: Instant) -> String {
        self.abbreviationAt(instant.secs)
    }

    // Whether daylight saving time is in effect at `instant`.
    public func isDaylightSavingTime(at instant: Instant) -> Bool {
        kestrel_tz_is_dst(self.id, instant.secs) != 0
    }

    // Every IANA zone name installed under /usr/share/zoneinfo, sorted.
    public static func availableIdentifiers() -> Array[String] {
        var probe = Array[UInt8](repeating: 0, count: 1);
        let needed = kestrel_tz_list_identifiers(probe.asPointer(), 0);
        var buf = Array[UInt8](repeating: 0, count: needed);
        kestrel_tz_list_identifiers(buf.asPointer(), needed);
        // Split the newline-separated list; the final byte is the NUL.
        var names: Array[String] = [];
        var start: Int64 = 0;
        var i: Int64 = 0;
        while i < needed - 1 {
            if buf(i) == 10 {
                names.append(stringFromBytes(buf, start, i - start));
                start = i + 1;
            }
            i = i + 1;
        }
        if i > start { names.append(stringFromBytes(buf, start, i - start)); }
        names
    }

    // --- Parsing ---

    // Parse a zone name: IANA ("America/New_York") or fixed offset ("+05:30").
    public static func parse(from input: String) -> TimeZone throws ParseError {
        guard let .Some(zone) = TimeZone(input) else {
            throw ParseError.InvalidValue("unknown time zone '\(input)'");
        }
        .Ok(zone)
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

    // The (offsetBefore, offsetAfter) pair of the DST transition whose gap or
    // fold window contains `dt`, or `.None` outside every window. The C side
    // binary-searches the zone's transition table (footer rules included).
    func findBracketingTransitions(dateTime dt: DateTime) -> (Int64, Int64)? {
        let (naiveSecs, _) = dt.toEpochSecs();
        var before: Int32 = 0;
        var after: Int32 = 0;
        if kestrel_tz_bracketing_transition(self.id, naiveSecs, Pointer(to: before), Pointer(to: after)) == 0 {
            return .None;
        }
        .Some((Int64(from: before), Int64(from: after)))
    }

    // Map a wall-clock time to epoch seconds in this zone — the one place DST
    // gaps and folds are resolved. Inside a transition window the candidates
    // are the wall time read with the offset on either side: in a fold both are
    // real instants; in a gap neither is, and they bracket the skipped span
    // (`.Earlier` = shifted back, `.Later` = shifted forward, as in Temporal).
    func resolveEpochSecs(dateTime dt: DateTime, naiveSecs naiveSecs: Int64,
                          disambiguation d: Disambiguation) -> Int64 {
        if let .Some((before, after)) = self.findBracketingTransitions(dateTime: dt) {
            let a = naiveSecs - before;
            let b = naiveSecs - after;
            let earlier = if a < b { a } else { b };
            let later = if a < b { b } else { a };
            let isFold = after < before;
            return match d {
                .Earlier => earlier,
                .Later => later,
                // RFC 5545: fold -> earlier, gap -> later
                .Compatible => if isFold { earlier } else { later }
            };
        }
        // Outside every transition window the wall time has exactly one
        // instant; a single refinement step lands on its offset.
        let guess = naiveSecs - self.offsetAt(naiveSecs);
        naiveSecs - self.offsetAt(guess)
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

    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.append(self.name);
    }
}
