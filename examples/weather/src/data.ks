// Data layer: JSON helpers and weather code mappings

module weather.data

import quill.value.(Value)

// ============================================================================
// JSON HELPERS
// ============================================================================

public func getFloat(v: Value) -> Float64 {
    match v.asFloat() {
        .Some(f) => f,
        .None => {
            match v.asInt() {
                .Some(n) => Float64(from: n),
                .None => 0.0
            }
        }
    }
}

public func getString(v: Value) -> String {
    match v.asString() {
        .Some(s) => s,
        .None => ""
    }
}

public func getInt(v: Value) -> Int64 {
    match v.asInt() {
        .Some(n) => n,
        .None => {
            match v.asFloat() {
                .Some(f) => {
                    match f.toInt64() {
                        .Some(n) => n,
                        .None => 0
                    }
                },
                .None => 0
            }
        }
    }
}

public func getField(obj: Value, key: String) -> Value {
    match obj.value(forKey: key) {
        .Some(v) => v,
        .None => Value.Null
    }
}

public func getArrayField(obj: Value, key: String) -> Array[Value] {
    match obj.value(forKey: key) {
        .Some(v) => {
            match v.asArray() {
                .Some(arr) => arr,
                .None => Array[Value]()
            }
        },
        .None => Array[Value]()
    }
}

public func getFloatFromArray(arr: Array[Value], idx: Int64) -> Float64 {
    if idx < arr.count {
        getFloat(arr(unchecked: idx))
    } else {
        0.0
    }
}

public func getIntFromArray(arr: Array[Value], idx: Int64) -> Int64 {
    if idx < arr.count {
        getInt(arr(unchecked: idx))
    } else {
        0
    }
}

// ============================================================================
// WEATHER CODES (WMO)
// ============================================================================

public func weatherEmoji(code: Int64) -> String {
    if code == 0 { return "☀️" }
    if code <= 3 { return "⛅" }
    if code <= 48 { return "🌫️" }
    if code <= 57 { return "🌦️" }
    if code <= 67 { return "🌧️" }
    if code <= 77 { return "🌨️" }
    if code <= 82 { return "🌧️" }
    if code <= 86 { return "🌨️" }
    "⛈️"
}

public func weatherDescription(code: Int64) -> String {
    if code == 0 { return "Clear sky" }
    if code == 1 { return "Mainly clear" }
    if code == 2 { return "Partly cloudy" }
    if code == 3 { return "Overcast" }
    if code <= 48 { return "Fog" }
    if code <= 55 { return "Drizzle" }
    if code <= 57 { return "Freezing drizzle" }
    if code <= 65 { return "Rain" }
    if code <= 67 { return "Freezing rain" }
    if code <= 75 { return "Snowfall" }
    if code == 77 { return "Snow grains" }
    if code <= 82 { return "Rain showers" }
    if code <= 86 { return "Snow showers" }
    if code == 95 { return "Thunderstorm" }
    "Thunderstorm with hail"
}

// ============================================================================
// WEATHER THEMING
// ============================================================================

public func weatherClass(code: Int64) -> String {
    if code == 0 { return "weather-sunny" }
    if code <= 3 { return "weather-cloudy" }
    if code <= 48 { return "weather-foggy" }
    if code <= 67 { return "weather-rainy" }
    if code <= 77 { return "weather-snowy" }
    if code <= 82 { return "weather-rainy" }
    if code <= 86 { return "weather-snowy" }
    "weather-stormy"
}

public func tempColorClass(temp: Float64) -> String {
    if temp < 32.0 { return "temp-freezing" }
    if temp < 50.0 { return "temp-cold" }
    if temp < 70.0 { return "temp-mild" }
    if temp < 85.0 { return "temp-warm" }
    "temp-hot"
}

public func evocativeDescription(code: Int64) -> String {
    if code == 0 { return "Clear skies, nothing but sun" }
    if code == 1 { return "Mostly clear with a few wisps" }
    if code == 2 { return "Sun and clouds playing tag" }
    if code == 3 { return "A thick blanket of clouds" }
    if code <= 48 { return "Mist hanging in the air" }
    if code <= 55 { return "A light drizzle falling" }
    if code <= 57 { return "Icy drizzle, watch your step" }
    if code <= 65 { return "Rain is pouring down" }
    if code <= 67 { return "Freezing rain, stay cozy" }
    if code <= 75 { return "Snow is gently falling" }
    if code == 77 { return "Tiny snow grains drifting" }
    if code <= 82 { return "Rain showers rolling through" }
    if code <= 86 { return "Snow showers blowing in" }
    if code == 95 { return "Thunder rumbling overhead" }
    "A wild thunderstorm with hail"
}

public func formatDateLabel(dateStr: String, idx: Int64) -> String {
    if idx == 0 { return "Today" }
    if idx == 1 { return "Tmrw" }
    if dateStr.byteCount >= 10 {
        return dateStr.substringBytes(from: 5, to: 10)
    };
    dateStr
}

// ============================================================================
// EXTENDED WEATHER HELPERS
// ============================================================================

public func getStringFromArray(arr: Array[Value], idx: Int64) -> String {
    if idx < arr.count {
        getString(arr(unchecked: idx))
    } else {
        ""
    }
}

public func parseHourFromIso(isoStr: String) -> Int64 {
    if isoStr.byteCount < 13 { return 0 };
    let hourStr = isoStr.substringBytes(from: 11, to: 13);
    match Int64.parse(hourStr) {
        .Some(n) => n,
        .None => 0
    }
}

public func formatHourLabel(timeStr: String, idx: Int64) -> String {
    if idx == 0 { return "Now" };
    let hour = parseHourFromIso(timeStr);
    if hour == 0 { return "12a" };
    if hour < 12 { return hour.format() + "a" };
    if hour == 12 { return "12p" };
    (hour - 12).format() + "p"
}

public func formatSunTime(isoStr: String) -> String {
    if isoStr.byteCount < 16 { return "" };
    let hourStr = isoStr.substringBytes(from: 11, to: 13);
    let minStr = isoStr.substringBytes(from: 14, to: 16);
    let hour = match Int64.parse(hourStr) {
        .Some(n) => n,
        .None => return ""
    };
    var s = String();
    if hour == 0 {
        s.append("12")
    } else if hour <= 12 {
        s.append(hour.format())
    } else {
        s.append((hour - 12).format())
    };
    s.append(":");
    s.append(minStr);
    if hour < 12 { s.append(" AM") } else { s.append(" PM") };
    s
}

public func uvDescription(idx: Int64) -> String {
    if idx <= 2 { return "Low" };
    if idx <= 5 { return "Moderate" };
    if idx <= 7 { return "High" };
    if idx <= 10 { return "Very High" };
    "Extreme"
}

public func pressureDescription(hpa: Float64) -> String {
    if hpa < 1000.0 { return "Low" };
    if hpa <= 1020.0 { return "Normal" };
    "High"
}

public func feelsLikeNote(feelsLike: Float64, actual: Float64) -> String {
    let diff = feelsLike - actual;
    if diff < -3.0 { return "Wind is making it feel cooler" };
    if diff > 3.0 { return "Humidity is making it feel warmer" };
    "Similar to the actual temperature"
}
