// Utilities: form parsing and number formatting

module weather.util

import http.url.(percentDecode)

// ============================================================================
// FORM PARSING
// ============================================================================

public func parseFormValue(body: String, key: String) -> String {
    // Parse "city=Berlin&other=val" style form data
    let keyEq = key + "=";
    let keyLen = keyEq.byteCount;
    let bodyLen = body.byteCount;

    // Find key= in the body
    var pos: Int64 = 0;
    while pos <= bodyLen - keyLen {
        var matched = true;
        var ki: Int64 = 0;
        while ki < keyLen {
            if body.bytes(unchecked: pos + ki) != keyEq.bytes(unchecked: ki) {
                matched = false;
                break
            };
            ki = ki + 1
        }
        if matched and (pos == 0 or body.bytes(unchecked: pos - 1) == 38) {
            // Found it — extract value until & or end
            let valStart = pos + keyLen;
            var valEnd = valStart;
            while valEnd < bodyLen and body.bytes(unchecked: valEnd) != 38 {
                valEnd = valEnd + 1
            }
            return percentDecode(body.asSlice().subslice(from: valStart, to: valEnd).toOwned())
        };
        pos = pos + 1
    }
    ""
}

// ============================================================================
// NUMBER FORMATTING
// ============================================================================

public func formatTemp(t: Float64) -> String {
    // Format to 1 decimal place using integer math
    let negative = t < 0.0;
    let absVal = if negative { 0.0 - t } else { t };
    let scaled = absVal * 10.0 + 0.5; // round
    let total = match scaled.toInt64() {
        .Some(n) => n,
        .None => 0
    };
    let whole = total / 10;
    let frac = total % 10;
    let sign = if negative and total > 0 { "-" } else { "" };
    "\(sign)\(whole).\(frac)"
}

public func formatInt(n: Int64) -> String {
    n.formatted()
}

public func formatTempWhole(t: Float64) -> String {
    let negative = t < 0.0;
    let absVal = if negative { 0.0 - t } else { t };
    let rounded = absVal + 0.5;
    let whole = match rounded.toInt64() {
        .Some(n) => n,
        .None => 0
    };
    let sign = if negative and whole > 0 { "-" } else { "" };
    "\(sign)\(whole)"
}
