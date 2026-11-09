module js

// Date (ECMA-262 §21.4). Local time is UTC for now: the engine has no
// time zone database, so LocalTZA is 0 everywhere.

import std.memory.(RawPointer)

@extern(.C, mangleName: "time")
func c_time(t: RawPointer) -> Int64

/// Milliseconds since the epoch, to the second.
func currentTime() -> Float64 {
    let seconds = unsafe c_time(RawPointer.nullPointer());
    Float64(from: seconds) * 1000.0
}

func dayOf(t: Float64) -> Float64 { (t / 86400000.0).floor() }

func timeWithinDay(t: Float64) -> Float64 {
    let r = fmod(t, 86400000.0);
    if r < 0.0 { r + 86400000.0 } else { r }
}

func dayFromYear(y: Float64) -> Float64 {
    365.0 * (y - 1970.0) + ((y - 1969.0) / 4.0).floor() - ((y - 1901.0) / 100.0).floor() + ((y - 1601.0) / 400.0).floor()
}

func daysInYear(y: Float64) -> Float64 {
    if fmod(y, 4.0) != 0.0 { return 365.0; }
    if fmod(y, 100.0) != 0.0 { return 366.0; }
    if fmod(y, 400.0) != 0.0 { return 365.0; }
    366.0
}

func yearFromTime(t: Float64) -> Float64 {
    // Estimate, then correct by whole years.
    var y = (t / (86400000.0 * 365.2425)).floor() + 1970.0;
    while dayFromYear(y) * 86400000.0 > t { y = y - 1.0; }
    while dayFromYear(y + 1.0) * 86400000.0 <= t { y = y + 1.0; }
    y
}

func inLeapYear(t: Float64) -> Bool { daysInYear(yearFromTime(t)) == 366.0 }

func dayWithinYear(t: Float64) -> Float64 { dayOf(t) - dayFromYear(yearFromTime(t)) }

/// The first day of each month in a common year, plus the year's end.
func monthStarts(leap: Bool) -> Array[Float64] {
    let l = if leap { 1.0 } else { 0.0 };
    [0.0, 31.0, 59.0 + l, 90.0 + l, 120.0 + l, 151.0 + l, 181.0 + l, 212.0 + l, 243.0 + l, 273.0 + l, 304.0 + l, 334.0 + l, 365.0 + l]
}

func monthFromTime(t: Float64) -> Float64 {
    let d = dayWithinYear(t);
    let starts = monthStarts(inLeapYear(t));
    var m: Int64 = 0;
    while m < 11 and d >= starts(m + 1) { m = m + 1; }
    Float64(from: m)
}

func dateFromTime(t: Float64) -> Float64 {
    let d = dayWithinYear(t);
    let starts = monthStarts(inLeapYear(t));
    let m = monthFromTime(t).toInt64() ?? 0;
    d - starts(m) + 1.0
}

func weekDay(t: Float64) -> Float64 {
    let r = fmod(dayOf(t) + 4.0, 7.0);
    if r < 0.0 { r + 7.0 } else { r }
}

func hourFromTime(t: Float64) -> Float64 { (timeWithinDay(t) / 3600000.0).floor() }
func minFromTime(t: Float64) -> Float64 { fmod((timeWithinDay(t) / 60000.0).floor(), 60.0) }
func secFromTime(t: Float64) -> Float64 { fmod((timeWithinDay(t) / 1000.0).floor(), 60.0) }
func msFromTime(t: Float64) -> Float64 { fmod(timeWithinDay(t), 1000.0) }

func integerPart(x: Float64) -> Float64 {
    let t = x.trunc();
    if t == 0.0 { 0.0 } else { t }
}

func makeTime(hour: Float64, min: Float64, sec: Float64, ms: Float64) -> Float64 {
    if not hour.isFinite or not min.isFinite or not sec.isFinite or not ms.isFinite { return Float64.nan; }
    integerPart(hour) * 3600000.0 + integerPart(min) * 60000.0 + integerPart(sec) * 1000.0 + integerPart(ms)
}

func makeDay(year: Float64, month: Float64, date: Float64) -> Float64 {
    if not year.isFinite or not month.isFinite or not date.isFinite { return Float64.nan; }
    let y = integerPart(year);
    let m = integerPart(month);
    let dt = integerPart(date);
    let ym = y + (m / 12.0).floor();
    if ym.abs() > 400000.0 { return Float64.nan; }
    let mn = fmod(m, 12.0);
    let mi = (if mn < 0.0 { mn + 12.0 } else { mn }).toInt64() ?? 0;
    let starts = monthStarts(daysInYear(ym) == 366.0);
    dayFromYear(ym) + starts(mi) + dt - 1.0
}

func makeDate(day: Float64, time: Float64) -> Float64 {
    if not day.isFinite or not time.isFinite { return Float64.nan; }
    let tv = day * 86400000.0 + time;
    if not tv.isFinite { return Float64.nan; }
    tv
}

func timeClip(t: Float64) -> Float64 {
    if not t.isFinite or t.abs() > 8640000000000000.0 { return Float64.nan; }
    integerPart(t)
}

func pad(n: Float64, width: Int64) -> String {
    var s = "\(n.toInt64() ?? 0)";
    while s.bytes.count < width { s = "0" + s; }
    s
}

func weekdayName(t: Float64) -> String {
    let names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    names(weekDay(t).toInt64() ?? 0)
}

func monthName(t: Float64) -> String {
    let names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    names(monthFromTime(t).toInt64() ?? 0)
}

/// The year for toString: at least four digits, `-` for negatives.
func yearText(t: Float64) -> String {
    let y = yearFromTime(t);
    if y < 0.0 { "-" + pad(0.0 - y, 4) } else { pad(y, 4) }
}

func dateString(t: Float64) -> String {
    weekdayName(t) + " " + monthName(t) + " " + pad(dateFromTime(t), 2) + " " + yearText(t)
}

func timeString(t: Float64) -> String {
    pad(hourFromTime(t), 2) + ":" + pad(minFromTime(t), 2) + ":" + pad(secFromTime(t), 2) + " GMT+0000 (Coordinated Universal Time)"
}

func isoString(t: Float64) -> String {
    let y = yearFromTime(t);
    let year: String = if y >= 0.0 and y <= 9999.0 { pad(y, 4) } else if y < 0.0 { "-" + pad(0.0 - y, 6) } else { "+" + pad(y, 6) };
    year + "-" + pad(monthFromTime(t) + 1.0, 2) + "-" + pad(dateFromTime(t), 2) + "T" + pad(hourFromTime(t), 2) + ":"
        + pad(minFromTime(t), 2) + ":" + pad(secFromTime(t), 2) + "." + pad(msFromTime(t), 3) + "Z"
}

func utcString(t: Float64) -> String {
    weekdayName(t) + ", " + pad(dateFromTime(t), 2) + " " + monthName(t) + " " + yearText(t) + " "
        + pad(hourFromTime(t), 2) + ":" + pad(minFromTime(t), 2) + ":" + pad(secFromTime(t), 2) + " GMT"
}

/// A cursor for Date.parse.
struct DateScanner {
    var s: Units
    var pos: Int64
}

func scanPeek(d: DateScanner) -> Int64 {
    if d.pos >= d.s.count { return -1; }
    Int64(from: d.s(d.pos))
}

/// Exactly `n` digits, or -1.
func scanDigits(mutating d: DateScanner, n: Int64) -> Float64 {
    var v: Int64 = 0;
    for _ in 0..<n {
        let c = scanPeek(d);
        if c < 48 or c > 57 { return -1.0; }
        v = v * 10 + c - 48;
        d.pos = d.pos + 1;
    }
    Float64(from: v)
}

/// One or more digits, or -1.
func scanNumber(mutating d: DateScanner) -> Float64 {
    let start = d.pos;
    var v: Int64 = 0;
    while scanPeek(d) >= 48 and scanPeek(d) <= 57 {
        v = v * 10 + scanPeek(d) - 48;
        d.pos = d.pos + 1;
    }
    if d.pos == start { -1.0 } else { Float64(from: v) }
}

func scanChar(mutating d: DateScanner, c: Int64) -> Bool {
    if scanPeek(d) != c { return false; }
    d.pos = d.pos + 1;
    true
}

/// Date.parse: the ISO format, then the toString and toUTCString forms.
func parseDate(s: Units) -> Float64 {
    let iso = parseISODate(s);
    if not iso.isNaN { return iso; }
    parseDateString(s)
}

/// `YYYY[-MM[-DD]][THH:mm[:ss[.sss]][Z|±HH:mm]]`, years as ±YYYYYY too.
func parseISODate(s: Units) -> Float64 {
    var d = DateScanner(s: s, pos: 0);
    var year: Float64 = 0.0;
    let c = scanPeek(d);
    if c == 43 or c == 45 {
        d.pos = d.pos + 1;
        year = scanDigits(d, 6);
        if year < 0.0 { return Float64.nan; }
        if c == 45 {
            if year == 0.0 { return Float64.nan; }
            year = 0.0 - year;
        }
    } else {
        year = scanDigits(d, 4);
        if year < 0.0 { return Float64.nan; }
    }
    var month: Float64 = 1.0;
    var day: Float64 = 1.0;
    if scanChar(d, 45) {
        month = scanDigits(d, 2);
        if month < 1.0 or month > 12.0 { return Float64.nan; }
        if scanChar(d, 45) {
            day = scanDigits(d, 2);
            if day < 1.0 or day > 31.0 { return Float64.nan; }
        }
    }
    var h: Float64 = 0.0;
    var mi: Float64 = 0.0;
    var sec: Float64 = 0.0;
    var ms: Float64 = 0.0;
    var offset: Float64 = 0.0;
    if scanChar(d, 84) {
        h = scanDigits(d, 2);
        if h < 0.0 or not scanChar(d, 58) { return Float64.nan; }
        mi = scanDigits(d, 2);
        if mi < 0.0 or mi > 59.0 { return Float64.nan; }
        if scanChar(d, 58) {
            sec = scanDigits(d, 2);
            if sec < 0.0 or sec > 59.0 { return Float64.nan; }
            if scanChar(d, 46) {
                // Any number of fraction digits; the first three count.
                let start = d.pos;
                var scale: Float64 = 100.0;
                while scanPeek(d) >= 48 and scanPeek(d) <= 57 {
                    ms = ms + Float64(from: scanPeek(d) - 48) * scale;
                    scale = scale / 10.0;
                    d.pos = d.pos + 1;
                }
                if d.pos == start { return Float64.nan; }
                ms = ms.floor();
            }
        }
        if h > 24.0 or (h == 24.0 and (mi > 0.0 or sec > 0.0 or ms > 0.0)) { return Float64.nan; }
        let z = scanPeek(d);
        if z == 90 {
            d.pos = d.pos + 1;
        } else if z == 43 or z == 45 {
            d.pos = d.pos + 1;
            let oh = scanDigits(d, 2);
            if oh < 0.0 or not scanChar(d, 58) { return Float64.nan; }
            let om = scanDigits(d, 2);
            if om < 0.0 or oh > 23.0 or om > 59.0 { return Float64.nan; }
            offset = (oh * 60.0 + om) * 60000.0;
            if z == 45 { offset = 0.0 - offset; }
        }
    }
    if d.pos != s.count { return Float64.nan; }
    let t = makeDate(makeDay(year, month - 1.0, day), makeTime(h, mi, sec, ms)) - offset;
    timeClip(t)
}

/// The forms toString and toUTCString produce:
/// `Tue Nov 09 2026 05:52:39 GMT+0000 (...)` and `Tue, 09 Nov 2026 05:52:39 GMT`.
func parseDateString(s: Units) -> Float64 {
    var words: [String] = [];
    var cur = "";
    for c in s {
        let x = Int64(from: c);
        if x == 32 or x == 44 {
            if not cur.isEmpty { words.append(cur); }
            cur = "";
        } else if x == 40 {
            break;
        } else {
            cur.append(char: Char(unchecked: UInt32(truncating: x)));
        }
    }
    if not cur.isEmpty { words.append(cur); }
    if words.count < 4 { return Float64.nan; }
    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    // Either "Wkd Mon DD YYYY ..." or "Wkd DD Mon YYYY ...".
    var month: Int64 = -1;
    var dayText = "";
    for i in 0..<12 {
        if words(1) == months(i) {
            month = i;
            dayText = words(2);
        } else if words(2) == months(i) {
            month = i;
            dayText = words(1);
        }
    }
    if month < 0 { return Float64.nan; }
    let day = parseIntText(dayText);
    let year = parseIntText(words(3));
    if day.isNaN or year.isNaN { return Float64.nan; }
    var h: Float64 = 0.0;
    var mi: Float64 = 0.0;
    var sec: Float64 = 0.0;
    if words.count > 4 {
        let parts = splitColon(words(4));
        if parts.count != 3 { return Float64.nan; }
        h = parseIntText(parts(0));
        mi = parseIntText(parts(1));
        sec = parseIntText(parts(2));
        if h.isNaN or mi.isNaN or sec.isNaN { return Float64.nan; }
    }
    timeClip(makeDate(makeDay(year, Float64(from: month), day), makeTime(h, mi, sec, 0.0)))
}

func splitColon(s: String) -> Array[String] {
    var out: [String] = [];
    var cur = "";
    for c in s.chars {
        if c == ':' {
            out.append(cur);
            cur = "";
        } else {
            cur.append(char: c);
        }
    }
    out.append(cur);
    out
}

/// A decimal integer with an optional minus, or NaN.
func parseIntText(s: String) -> Float64 {
    var v: Float64 = 0.0;
    var negative = false;
    var any = false;
    for c in s.chars {
        if c == '-' and not any and not negative {
            negative = true;
            continue;
        }
        let x = Int64(from: c.value());
        if x < 48 or x > 57 { return Float64.nan; }
        v = v * 10.0 + Float64(from: x - 48);
        any = true;
    }
    if not any { return Float64.nan; }
    if negative { 0.0 - v } else { v }
}

extend VM {
    mutating func setupDate() {
        let proto = self.newObject();
        let ctor = self.constructorFor("Date", .DateCtor, 7, proto);
        self.method(ctor, "now", .DateNow, 0);
        self.method(ctor, "parse", .DateParse, 1);
        self.method(ctor, "UTC", .DateUTC, 7);
        for (name, n, len) in [
            ("getTime", NativeFn.DateGetTime, 0), ("valueOf", NativeFn.DateGetTime, 0),
            ("getFullYear", NativeFn.DateGetFullYear, 0), ("getUTCFullYear", NativeFn.DateGetFullYear, 0),
            ("getMonth", NativeFn.DateGetMonth, 0), ("getUTCMonth", NativeFn.DateGetMonth, 0),
            ("getDate", NativeFn.DateGetDate, 0), ("getUTCDate", NativeFn.DateGetDate, 0),
            ("getDay", NativeFn.DateGetDay, 0), ("getUTCDay", NativeFn.DateGetDay, 0),
            ("getHours", NativeFn.DateGetHours, 0), ("getUTCHours", NativeFn.DateGetHours, 0),
            ("getMinutes", NativeFn.DateGetMinutes, 0), ("getUTCMinutes", NativeFn.DateGetMinutes, 0),
            ("getSeconds", NativeFn.DateGetSeconds, 0), ("getUTCSeconds", NativeFn.DateGetSeconds, 0),
            ("getMilliseconds", NativeFn.DateGetMilliseconds, 0), ("getUTCMilliseconds", NativeFn.DateGetMilliseconds, 0),
            ("getTimezoneOffset", NativeFn.DateGetTimezoneOffset, 0), ("getYear", NativeFn.DateGetYear, 0),
            ("setTime", NativeFn.DateSetTime, 1),
            ("setMilliseconds", NativeFn.DateSetMilliseconds, 1), ("setUTCMilliseconds", NativeFn.DateSetMilliseconds, 1),
            ("setSeconds", NativeFn.DateSetSeconds, 2), ("setUTCSeconds", NativeFn.DateSetSeconds, 2),
            ("setMinutes", NativeFn.DateSetMinutes, 3), ("setUTCMinutes", NativeFn.DateSetMinutes, 3),
            ("setHours", NativeFn.DateSetHours, 4), ("setUTCHours", NativeFn.DateSetHours, 4),
            ("setDate", NativeFn.DateSetDate, 1), ("setUTCDate", NativeFn.DateSetDate, 1),
            ("setMonth", NativeFn.DateSetMonth, 2), ("setUTCMonth", NativeFn.DateSetMonth, 2),
            ("setFullYear", NativeFn.DateSetFullYear, 3), ("setUTCFullYear", NativeFn.DateSetFullYear, 3),
            ("setYear", NativeFn.DateSetYear, 1),
            ("toISOString", NativeFn.DateToISOString, 0), ("toJSON", NativeFn.DateToJSON, 1),
            ("toString", NativeFn.DateToString, 0), ("toDateString", NativeFn.DateToDateString, 0),
            ("toTimeString", NativeFn.DateToTimeString, 0), ("toUTCString", NativeFn.DateToUTCString, 0),
            ("toLocaleString", NativeFn.DateToString, 0), ("toLocaleDateString", NativeFn.DateToDateString, 0),
            ("toLocaleTimeString", NativeFn.DateToTimeString, 0)
        ] {
            self.method(proto, name, n, len);
        }
        // toGMTString is the same function as toUTCString.
        let utc = self.ownValue(proto.asRef, self.atom("toUTCString"));
        self.defineHidden(proto.asRef, self.atom("toGMTString"), utc);
        let toPrim = self.nativeFunction(.DateToPrimitive, "[Symbol.toPrimitive]", 1, false);
        self.defineOwn(proto.asRef, symbolKey(WellKnown.toPrimitive), toPrim, Attr.configurable);
    }

    /// `this`'s time value, for Date.prototype methods.
    func thisTime(this: Value) -> Optional[Float64] {
        if not this.isObject { return .None; }
        if let .DateObject(t) = self.kindAt(this.asRef) { return .Some(t); }
        .None
    }

    mutating func requireTime(this: Value) -> Result[Float64, Value] {
        match self.thisTime(this) {
            .Some(t) => .Ok(t),
            .None => self.throwType("this is not a Date object.")
        }
    }

    mutating func setTime(this: Value, t: Float64) -> Value {
        let v = timeClip(t);
        self.heap.objects(this.asRef).kind = .DateObject(v);
        Value.number(v)
    }

    /// The numbers in `args`, converted in order (each conversion can
    /// run user code, so all of them happen before any use).
    mutating func numberArgs(args: Array[Value], count: Int64) -> Result[Array[Float64], Value] {
        var out: [Float64] = [];
        for i in 0..<count {
            if i >= args.count { break; }
            out.append(try self.toNumber(args(i)));
        }
        .Ok(out)
    }

    mutating func dateNative(n: NativeFn, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let r: Value = match n {
            .DateCtor => {
                if newTarget.isUndefined { return .Ok(.Some(self.str(dateString(currentTime()) + " " + timeString(currentTime())))); }
                var tv = currentTime();
                if args.count == 1 {
                    if let .Some(t) = self.thisTime(a0) {
                        tv = t;
                    } else {
                        let p = try self.toPrimitive(a0, "default");
                        tv = if p.isString { parseDate(self.units(p)) } else { try self.toNumber(p) };
                    }
                } else if args.count > 1 {
                    let ns = try self.numberArgs(args, 7);
                    tv = dateFromParts(ns);
                }
                let proto = try self.prototypeFor(newTarget, self.ownValue(self.ownValue(self.global.asRef, self.atom("Date")).asRef, self.names.prototype));
                self.newObjectKind(proto, .DateObject(timeClip(tv)))
            },
            .DateNow => Value.number(currentTime()),
            .DateParse => Value.number(parseDate(try self.toUnits(a0))),
            .DateUTC => {
                let ns = try self.numberArgs(args, 7);
                if ns.count == 0 { Value.number(Float64.nan) } else { Value.number(timeClip(dateFromParts(ns))) }
            },
            .DateGetTime => Value.number(try self.requireTime(this)),
            .DateGetFullYear => self.datePart(try self.requireTime(this), 0),
            .DateGetMonth => self.datePart(try self.requireTime(this), 1),
            .DateGetDate => self.datePart(try self.requireTime(this), 2),
            .DateGetDay => self.datePart(try self.requireTime(this), 3),
            .DateGetHours => self.datePart(try self.requireTime(this), 4),
            .DateGetMinutes => self.datePart(try self.requireTime(this), 5),
            .DateGetSeconds => self.datePart(try self.requireTime(this), 6),
            .DateGetMilliseconds => self.datePart(try self.requireTime(this), 7),
            .DateGetTimezoneOffset => {
                let t = try self.requireTime(this);
                Value.number(if t.isNaN { t } else { 0.0 })
            },
            .DateGetYear => {
                let t = try self.requireTime(this);
                Value.number(if t.isNaN { t } else { yearFromTime(t) - 1900.0 })
            },
            .DateSetTime => {
                let _ = try self.requireTime(this);
                let t = try self.toNumber(a0);
                self.setTime(this, t)
            },
            .DateSetMilliseconds => {
                let t = try self.requireTime(this);
                let ms = try self.toNumber(a0);
                if t.isNaN { return .Ok(.Some(Value.number(t))); }
                self.setTime(this, makeDate(dayOf(t), makeTime(hourFromTime(t), minFromTime(t), secFromTime(t), ms)))
            },
            .DateSetSeconds => {
                let t = try self.requireTime(this);
                let ns = try self.numberArgs(args, 2);
                if t.isNaN { return .Ok(.Some(Value.number(t))); }
                let s = argOr(ns, 0, Float64.nan);
                let ms = argOr(ns, 1, msFromTime(t));
                self.setTime(this, makeDate(dayOf(t), makeTime(hourFromTime(t), minFromTime(t), s, ms)))
            },
            .DateSetMinutes => {
                let t = try self.requireTime(this);
                let ns = try self.numberArgs(args, 3);
                if t.isNaN { return .Ok(.Some(Value.number(t))); }
                let m = argOr(ns, 0, Float64.nan);
                let s = argOr(ns, 1, secFromTime(t));
                let ms = argOr(ns, 2, msFromTime(t));
                self.setTime(this, makeDate(dayOf(t), makeTime(hourFromTime(t), m, s, ms)))
            },
            .DateSetHours => {
                let t = try self.requireTime(this);
                let ns = try self.numberArgs(args, 4);
                if t.isNaN { return .Ok(.Some(Value.number(t))); }
                let h = argOr(ns, 0, Float64.nan);
                let m = argOr(ns, 1, minFromTime(t));
                let s = argOr(ns, 2, secFromTime(t));
                let ms = argOr(ns, 3, msFromTime(t));
                self.setTime(this, makeDate(dayOf(t), makeTime(h, m, s, ms)))
            },
            .DateSetDate => {
                let t = try self.requireTime(this);
                let dt = try self.toNumber(a0);
                if t.isNaN { return .Ok(.Some(Value.number(t))); }
                self.setTime(this, makeDate(makeDay(yearFromTime(t), monthFromTime(t), dt), timeWithinDay(t)))
            },
            .DateSetMonth => {
                let t = try self.requireTime(this);
                let ns = try self.numberArgs(args, 2);
                if t.isNaN { return .Ok(.Some(Value.number(t))); }
                let m = argOr(ns, 0, Float64.nan);
                let dt = argOr(ns, 1, dateFromTime(t));
                self.setTime(this, makeDate(makeDay(yearFromTime(t), m, dt), timeWithinDay(t)))
            },
            .DateSetFullYear => {
                let t0 = try self.requireTime(this);
                let ns = try self.numberArgs(args, 3);
                let t = if t0.isNaN { 0.0 } else { t0 };
                let y = argOr(ns, 0, Float64.nan);
                let m = argOr(ns, 1, monthFromTime(t));
                let dt = argOr(ns, 2, dateFromTime(t));
                self.setTime(this, makeDate(makeDay(y, m, dt), timeWithinDay(t)))
            },
            .DateSetYear => {
                let t0 = try self.requireTime(this);
                let y = try self.toNumber(a0);
                if y.isNaN { return .Ok(.Some(self.setTime(this, Float64.nan))); }
                let t = if t0.isNaN { 0.0 } else { t0 };
                let yi = integerPart(y);
                let full = if yi >= 0.0 and yi <= 99.0 { yi + 1900.0 } else { y };
                self.setTime(this, makeDate(makeDay(full, monthFromTime(t), dateFromTime(t)), timeWithinDay(t)))
            },
            .DateToISOString => {
                let t = try self.requireTime(this);
                if t.isNaN { return .Err(self.makeError(.RangeError, "Invalid time value")); }
                self.str(isoString(t))
            },
            .DateToJSON => {
                let o = try self.toObject(this);
                let tv = try self.toPrimitive(o, "number");
                if tv.isNumber and not tv.asNumber.isFinite { return .Ok(.Some(Value.nullValue)); }
                let f = try self.getV(o, self.atom("toISOString"));
                if not self.isCallable(f) { return self.throwType("toISOString is not a function"); }
                try self.callValue(f, o, [])
            },
            .DateToString => self.dateText(try self.requireTime(this), 0),
            .DateToDateString => self.dateText(try self.requireTime(this), 1),
            .DateToTimeString => self.dateText(try self.requireTime(this), 2),
            .DateToUTCString => self.dateText(try self.requireTime(this), 3),
            .DateToPrimitive => {
                if not this.isObject { return self.throwType("Date.prototype[Symbol.toPrimitive] called on non-object"); }
                let hint = if a0.isString { self.text(a0) } else { "" };
                if hint == "default" or hint == "string" {
                    try self.ordinaryToPrimitive(this, "string")
                } else if hint == "number" {
                    try self.ordinaryToPrimitive(this, "number")
                } else {
                    return self.throwType("Invalid hint");
                }
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    /// One field of a time value: year, month, date, weekday, hours,
    /// minutes, seconds or milliseconds.
    func datePart(t: Float64, which: Int64) -> Value {
        if t.isNaN { return Value.number(t); }
        let parts = [yearFromTime(t), monthFromTime(t), dateFromTime(t), weekDay(t),
                     hourFromTime(t), minFromTime(t), secFromTime(t), msFromTime(t)];
        Value.number(parts(which))
    }

    mutating func dateText(t: Float64, which: Int64) -> Value {
        if t.isNaN { return self.str("Invalid Date"); }
        let s = if which == 0 { dateString(t) + " " + timeString(t) }
            else if which == 1 { dateString(t) }
            else if which == 2 { timeString(t) }
            else { utcString(t) };
        self.str(s)
    }
}

func argOr(ns: Array[Float64], i: Int64, fallback: Float64) -> Float64 {
    if i < ns.count { ns(i) } else { fallback }
}

/// A time value from (year, month[, date, hours, minutes, seconds, ms]),
/// with two-digit years meaning 19xx.
func dateFromParts(ns: Array[Float64]) -> Float64 {
    var y = ns(0);
    if not y.isNaN {
        let yi = integerPart(y);
        if yi >= 0.0 and yi <= 99.0 { y = 1900.0 + yi; }
    }
    let m = argOr(ns, 1, 0.0);
    let dt = argOr(ns, 2, 1.0);
    let h = argOr(ns, 3, 0.0);
    let mi = argOr(ns, 4, 0.0);
    let s = argOr(ns, 5, 0.0);
    let ms = argOr(ns, 6, 0.0);
    makeDate(makeDay(y, m, dt), makeTime(h, mi, s, ms))
}
