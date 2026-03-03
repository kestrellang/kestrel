// UI layer: all HTML generation

module weather.ui

import quill.value.(Value)
import plume.plume.(Template)
import weather.data.(getFloat, getString, getInt, getField, getArrayField, getFloatFromArray, getIntFromArray, weatherEmoji, weatherDescription, weatherClass, tempColorClass, evocativeDescription, formatDateLabel)
import weather.util.(urlEncode, formatTemp, formatTempWhole, formatInt)

// ============================================================================
// SHARED CSS
// ============================================================================

func baseCss() -> String {
    var s = String(capacity: 3072);
    s.append("*{box-sizing:border-box;margin:0;padding:0}");
    s.append("body{font-family:'Inter',system-ui,-apple-system,sans-serif;color:#f1f0f5;min-height:100vh;-webkit-font-smoothing:antialiased}");
    s.append("a{color:inherit;text-decoration:none}");
    // Search wrapper (shared between landing + header)
    s.append(".search-wrap{position:relative}");
    s.append(".search-wrap input{width:100%;padding:16px 20px;border-radius:16px;border:1px solid rgba(255,255,255,0.1);background:rgba(255,255,255,0.06);backdrop-filter:blur(12px);-webkit-backdrop-filter:blur(12px);color:#f1f0f5;font-size:1rem;font-family:inherit;outline:none;transition:border-color 0.3s,box-shadow 0.3s,background 0.3s}");
    s.append(".search-wrap input::placeholder{color:#6b6784}");
    s.append(".search-wrap input:focus{border-color:rgba(167,139,250,0.5);box-shadow:0 0 0 3px rgba(167,139,250,0.15);background:rgba(255,255,255,0.09)}");
    // Dropdown
    s.append(".dropdown{position:absolute;left:0;right:0;top:100%;margin-top:8px;z-index:100;max-height:320px;overflow-y:auto;border-radius:16px}");
    s.append(".dropdown:empty{display:none}");
    s.append(".dropdown:not(:empty){background:rgba(20,20,40,0.95);backdrop-filter:blur(20px);-webkit-backdrop-filter:blur(20px);border:1px solid rgba(255,255,255,0.1);box-shadow:0 16px 48px rgba(0,0,0,0.5);padding:6px}");
    // City items in dropdown
    s.append(".city-item{display:block;padding:14px 16px;border-radius:12px;cursor:pointer;transition:all 0.2s ease;animation:fadeSlideIn 0.25s ease both}");
    s.append(".city-item:hover{background:rgba(167,139,250,0.12)}");
    s.append(".city-name{font-weight:600;color:#f1f0f5;font-size:0.95rem}");
    s.append(".city-detail{font-size:0.8rem;color:#8b87a0;margin-top:2px}");
    // Error
    s.append(".error{background:rgba(239,68,68,0.1);border:1px solid rgba(239,68,68,0.2);border-radius:14px;padding:16px 20px;color:#fca5a5;text-align:center;font-size:0.9rem;animation:fadeSlideIn 0.3s ease both}");
    // Loading indicator
    s.append(".htmx-indicator{display:none}");
    s.append(".htmx-request .htmx-indicator{display:block}");
    s.append(".search-spinner{position:absolute;right:16px;top:50%;transform:translateY(-50%);width:18px;height:18px;border:2px solid rgba(167,139,250,0.2);border-top-color:#a78bfa;border-radius:50%;animation:spin 0.7s linear infinite}");
    // Animations
    s.append("@keyframes spin{to{transform:rotate(360deg)}}");
    s.append("@keyframes fadeSlideIn{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:translateY(0)}}");
    s.append("@keyframes float{0%,100%{transform:translateY(0)}50%{transform:translateY(-8px)}}");
    s
}

// ============================================================================
// HTML: LANDING PAGE
// ============================================================================

public func pageHtml() -> String {
    var h = String(capacity: 4096);
    h.append("<!DOCTYPE html><html><head>");
    h.append("<meta charset=\"utf-8\">");
    h.append("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    h.append("<title>Kestrel Weather</title>");
    h.append("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">");
    h.append("<link href=\"https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&display=swap\" rel=\"stylesheet\">");
    h.append("<script src=\"https://unpkg.com/htmx.org@1.9.10\"></script>");
    h.append("<style>");
    h.append(baseCss());
    h.append("body{background:linear-gradient(160deg,#0f0f23 0%,#1a1a3e 40%,#2d1b4e 100%);background-attachment:fixed}");
    h.append(".landing{display:flex;flex-direction:column;align-items:center;justify-content:center;min-height:100vh;padding:40px 24px}");
    h.append(".hero{text-align:center;margin-bottom:40px}");
    h.append(".hero-emoji{font-size:4.5rem;margin-bottom:20px;animation:float 4s ease-in-out infinite}");
    h.append(".hero h1{font-size:2.2rem;font-weight:700;letter-spacing:-0.02em;color:#fff;margin-bottom:8px}");
    h.append(".hero .subtitle{font-size:1rem;color:#8b87a0;letter-spacing:0.02em}");
    h.append(".landing .search-wrap{width:100%;max-width:480px}");
    h.append(".landing .search-wrap input{padding:20px 24px;border-radius:20px;font-size:1.1rem}");
    h.append(".landing .dropdown{border-radius:20px}");
    h.append("@media(max-width:640px){.hero h1{font-size:1.6rem}.hero-emoji{font-size:3.5rem}.landing .search-wrap input{padding:16px 20px;font-size:1rem}}");
    h.append("</style></head><body>");
    h.append("<div class=\"landing\">");
    h.append("<div class=\"hero\">");
    h.append("<div class=\"hero-emoji\">&#x1F324;&#xFE0F;</div>");
    h.append("<h1>Kestrel Weather</h1>");
    h.append("<p class=\"subtitle\">What's the sky up to?</p>");
    h.append("</div>");
    h.append("<div class=\"search-wrap\">");
    h.append("<input type=\"text\" name=\"q\" placeholder=\"Search for a city...\" autocomplete=\"off\" autofocus");
    h.append(" hx-get=\"/search\" hx-trigger=\"keyup changed delay:300ms\" hx-target=\"#dropdown\">");
    h.append("<div class=\"htmx-indicator\"><div class=\"search-spinner\"></div></div>");
    h.append("<div id=\"dropdown\" class=\"dropdown\"></div>");
    h.append("</div>");
    h.append("</div></body></html>");
    h
}

// ============================================================================
// HTML: SEARCH RESULTS (dropdown items)
// ============================================================================

public func searchResultsHtml(json: Value) -> String {
    var h = String();

    match json.value(forKey: "results") {
        .Some(resultsVal) => {
            match resultsVal.asArray() {
                .Some(results) => {
                    if results.count == 0 {
                        h.append("<div class=\"error\">No cities found. Try a different search.</div>");
                        return h
                    };
                    var t = Template();
                    var i: Int64 = 0;
                    while i < results.count {
                        let city = results(unchecked: i);
                        let name = getString(getField(city, "name"));
                        let country = getString(getField(city, "country"));
                        let admin1 = getString(getField(city, "admin1"));
                        let lat = getFloat(getField(city, "latitude"));
                        let lon = getFloat(getField(city, "longitude"));

                        t.setRaw("lat", lat.format());
                        t.setRaw("lon", lon.format());
                        t.put("name", name);
                        t.setRaw("encodedName", urlEncode(name));
                        t.setInt("delay", i * 40);

                        var detail = String();
                        if admin1.byteCount > 0 {
                            detail.append(admin1);
                            detail.append(", ")
                        };
                        detail.append(country);
                        t.put("detail", detail);

                        var itemTpl = String(capacity: 256);
                        itemTpl.append("<a class=\"city-item\" style=\"animation-delay:{delay}ms\" href=\"/weather?lat={lat}&lon={lon}&name={encodedName}\">");
                        itemTpl.append("<div class=\"city-name\">{name}</div>");
                        itemTpl.append("<div class=\"city-detail\">{detail}</div></a>");
                        h.append(t.render(itemTpl));
                        i = i + 1
                    }
                },
                .None => {
                    h.append("<div class=\"error\">Unexpected response format.</div>")
                }
            }
        },
        .None => {
            h.append("<div class=\"error\">No results found.</div>")
        }
    }
    h
}

// ============================================================================
// HTML: WEATHER PAGE (full page)
// ============================================================================

public func weatherPageHtml(json: Value, cityName: String) -> String {
    var h = String();
    var t = Template();

    let current = getField(json, "current");
    let daily = getField(json, "daily");

    let temp = getFloat(getField(current, "temperature_2m"));
    let code = getInt(getField(current, "weather_code"));
    let wind = getFloat(getField(current, "wind_speed_10m"));
    let humidity = getFloat(getField(current, "relative_humidity_2m"));
    let humInt = match humidity.toInt64() {
        .Some(n) => n,
        .None => 0
    };

    let wClass = weatherClass(code);

    // Start building full page
    h.append("<!DOCTYPE html><html><head>");
    h.append("<meta charset=\"utf-8\">");
    h.append("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");

    t.put("city", cityName);
    h.append(t.render("<title>{city} — Kestrel Weather</title>"));

    h.append("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">");
    h.append("<link href=\"https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&display=swap\" rel=\"stylesheet\">");
    h.append("<script src=\"https://unpkg.com/htmx.org@1.9.10\"></script>");
    h.append("<style>");
    h.append(baseCss());
    h.append(weatherPageCss());
    h.append("</style></head>");

    // Body with weather-conditional class
    t.setRaw("wClass", wClass);
    h.append(t.render("<body class=\"{wClass}\">"));

    // Header with search
    h.append("<header class=\"header\">");
    h.append("<a href=\"/\" class=\"logo\">Kestrel Weather</a>");
    h.append("<div class=\"search-wrap header-search\">");
    h.append("<input type=\"text\" name=\"q\" placeholder=\"Search city...\" autocomplete=\"off\"");
    h.append(" hx-get=\"/search\" hx-trigger=\"keyup changed delay:300ms\" hx-target=\"#dropdown\">");
    h.append("<div class=\"htmx-indicator\"><div class=\"search-spinner\"></div></div>");
    h.append("<div id=\"dropdown\" class=\"dropdown\"></div>");
    h.append("</div></header>");

    // Weather hero
    t.put("city", cityName);
    t.setRaw("desc", evocativeDescription(code));
    t.setRaw("emoji", weatherEmoji(code));
    t.setRaw("temp", formatTempWhole(temp));
    t.setRaw("wind", formatTemp(wind));
    t.setRaw("humidity", formatInt(humInt));

    var heroTpl = String(capacity: 512);
    heroTpl.append("<main class=\"weather-main\">");
    heroTpl.append("<div class=\"weather-hero\">");
    heroTpl.append("<div class=\"weather-emoji\">{emoji}</div>");
    heroTpl.append("<div class=\"weather-temp\">{temp}<span class=\"weather-unit\">&deg;</span></div>");
    heroTpl.append("<div class=\"weather-city\">{city}</div>");
    heroTpl.append("<div class=\"weather-desc\">{desc}</div>");
    heroTpl.append("</div>");
    heroTpl.append("<div class=\"weather-stats\">");
    heroTpl.append("<div class=\"stat\"><div class=\"stat-icon\">&#x1F4A8;</div><div><div class=\"stat-value\">{wind} mph</div><div class=\"stat-label\">Wind</div></div></div>");
    heroTpl.append("<div class=\"stat\"><div class=\"stat-icon\">&#x1F4A7;</div><div><div class=\"stat-value\">{humidity}%</div><div class=\"stat-label\">Humidity</div></div></div>");
    heroTpl.append("</div>");
    h.append(t.render(heroTpl));

    // 7-day forecast
    h.append("<div class=\"forecast-section\">");
    h.append("<div class=\"forecast-title\">This Week</div>");
    h.append("<div class=\"forecast\">");

    match daily.value(forKey: "time") {
        .Some(timesVal) => {
            match timesVal.asArray() {
                .Some(times) => {
                    let codes = getArrayField(daily, "weather_code");
                    let highs = getArrayField(daily, "temperature_2m_max");
                    let lows = getArrayField(daily, "temperature_2m_min");
                    let rain = getArrayField(daily, "precipitation_sum");

                    var i: Int64 = 0;
                    while i < times.count {
                        let dateStr = getString(times(unchecked: i));
                        let dayCode = getIntFromArray(codes, i);
                        let high = getFloatFromArray(highs, i);
                        let low = getFloatFromArray(lows, i);
                        let precip = getFloatFromArray(rain, i);

                        let dateLabel = formatDateLabel(dateStr, i);
                        let tClass = tempColorClass(high);
                        let todayClass = if i == 0 { "forecast-day forecast-today" } else { "forecast-day" };

                        t.setRaw("dateLabel", dateLabel);
                        t.setRaw("icon", weatherEmoji(dayCode));
                        t.setRaw("high", formatTempWhole(high));
                        t.setRaw("low", formatTempWhole(low));
                        t.setRaw("tClass", tClass);
                        t.setRaw("todayClass", todayClass);
                        t.setInt("delay", i * 50);

                        var precipHtml = String();
                        if precip > 0.0 {
                            t.setRaw("precip", formatTemp(precip));
                            precipHtml = t.render("<div class=\"forecast-rain\">{precip}\"</div>")
                        };
                        t.setRaw("precipHtml", precipHtml);

                        var dayTpl = String(capacity: 256);
                        dayTpl.append("<div class=\"{todayClass}\" style=\"animation-delay:{delay}ms\">");
                        dayTpl.append("<div class=\"forecast-date\">{dateLabel}</div>");
                        dayTpl.append("<div class=\"forecast-emoji\">{icon}</div>");
                        dayTpl.append("<div class=\"forecast-high {tClass}\">{high}&deg;</div>");
                        dayTpl.append("<div class=\"forecast-low\">{low}&deg;</div>");
                        dayTpl.append("{precipHtml}</div>");
                        h.append(t.render(dayTpl));
                        i = i + 1
                    }
                },
                .None => {}
            }
        },
        .None => {}
    }

    h.append("</div></div>");  // close .forecast + .forecast-section
    h.append("</main>");
    h.append("</body></html>");
    h
}

func weatherPageCss() -> String {
    var s = String(capacity: 3584);
    // Weather page backgrounds per condition
    s.append("body.weather-sunny{background:linear-gradient(160deg,#1a1508 0%,#2d2210 40%,#3d2a08 100%)}");
    s.append("body.weather-cloudy{background:linear-gradient(160deg,#121218 0%,#1a1a28 40%,#222230 100%)}");
    s.append("body.weather-rainy{background:linear-gradient(160deg,#0a1520 0%,#0f2030 40%,#0a1828 100%)}");
    s.append("body.weather-snowy{background:linear-gradient(160deg,#0f1318 0%,#151d28 40%,#1a2530 100%)}");
    s.append("body.weather-stormy{background:linear-gradient(160deg,#150a20 0%,#201030 40%,#2a1540 100%)}");
    s.append("body.weather-foggy{background:linear-gradient(160deg,#111114 0%,#18181e 40%,#1e1e24 100%)}");
    // Header
    s.append(".header{display:flex;align-items:center;justify-content:space-between;padding:16px 24px;border-bottom:1px solid rgba(255,255,255,0.06)}");
    s.append(".logo{font-weight:600;font-size:0.95rem;color:#8b87a0;letter-spacing:0.02em;transition:color 0.2s}");
    s.append(".logo:hover{color:#f1f0f5}");
    s.append(".header-search{width:260px}");
    s.append(".header-search input{padding:10px 16px;border-radius:12px;font-size:0.9rem}");
    // Main content
    s.append(".weather-main{max-width:680px;margin:0 auto;padding:48px 24px}");
    // Hero section
    s.append(".weather-hero{text-align:center;margin-bottom:40px;animation:fadeSlideIn 0.5s ease both}");
    s.append(".weather-emoji{font-size:5rem;margin-bottom:8px;animation:float 3s ease-in-out infinite}");
    s.append(".weather-temp{font-size:8rem;font-weight:800;color:#fff;line-height:1;letter-spacing:-0.05em;margin-bottom:8px}");
    s.append(".weather-unit{font-size:2.5rem;color:#8b87a0;font-weight:400}");
    s.append(".weather-city{font-size:1.8rem;font-weight:700;color:#f1f0f5;margin-bottom:4px}");
    s.append(".weather-desc{font-size:1.05rem;color:#8b87a0;font-weight:400}");
    // Stats
    s.append(".weather-stats{display:flex;justify-content:center;gap:24px;margin-bottom:48px;animation:fadeSlideIn 0.5s ease 0.1s both}");
    s.append(".stat{display:flex;align-items:center;gap:12px;background:rgba(255,255,255,0.05);backdrop-filter:blur(8px);-webkit-backdrop-filter:blur(8px);border-radius:16px;padding:16px 24px;border:1px solid rgba(255,255,255,0.06)}");
    s.append(".stat-icon{font-size:1.4rem}");
    s.append(".stat-value{font-size:1.1rem;font-weight:700;color:#f1f0f5}");
    s.append(".stat-label{font-size:0.72rem;color:#8b87a0;text-transform:uppercase;letter-spacing:0.06em;font-weight:500;margin-top:1px}");
    // Forecast
    s.append(".forecast-section{animation:fadeSlideIn 0.5s ease 0.2s both}");
    s.append(".forecast-title{font-size:0.8rem;color:#8b87a0;text-transform:uppercase;letter-spacing:0.1em;font-weight:600;margin-bottom:16px}");
    s.append(".forecast{display:grid;grid-template-columns:repeat(7,1fr);gap:8px}");
    s.append(".forecast-day{background:rgba(255,255,255,0.04);backdrop-filter:blur(8px);-webkit-backdrop-filter:blur(8px);border-radius:16px;padding:16px 8px;text-align:center;border:1px solid rgba(255,255,255,0.05);transition:all 0.2s ease;animation:fadeSlideIn 0.3s ease both}");
    s.append(".forecast-day:hover{background:rgba(255,255,255,0.08);transform:translateY(-2px);box-shadow:0 4px 16px rgba(0,0,0,0.2)}");
    s.append(".forecast-today{border-color:rgba(167,139,250,0.2);background:rgba(167,139,250,0.06)}");
    s.append(".forecast-date{font-size:0.72rem;color:#8b87a0;margin-bottom:10px;font-weight:600;text-transform:uppercase;letter-spacing:0.04em}");
    s.append(".forecast-today .forecast-date{color:#a78bfa}");
    s.append(".forecast-emoji{font-size:1.5rem;margin-bottom:10px;display:block}");
    s.append(".forecast-high{font-weight:700;font-size:1rem}");
    s.append(".forecast-low{color:#6b6784;font-size:0.85rem;margin-top:3px}");
    s.append(".forecast-rain{font-size:0.72rem;color:#67e8f9;margin-top:6px;font-weight:500}");
    // Temp color classes
    s.append(".temp-freezing{color:#93c5fd}");
    s.append(".temp-cold{color:#67e8f9}");
    s.append(".temp-mild{color:#6ee7b7}");
    s.append(".temp-warm{color:#fbbf24}");
    s.append(".temp-hot{color:#f87171}");
    // Responsive
    s.append("@media(max-width:640px){.header{padding:12px 16px}.header-search{width:200px}.header-search input{padding:8px 12px;font-size:0.85rem}.weather-main{padding:32px 16px}.weather-temp{font-size:5.5rem}.weather-emoji{font-size:3.5rem}.weather-stats{flex-direction:column;align-items:center}.stat{width:100%;max-width:280px}.forecast{grid-template-columns:repeat(4,1fr);gap:6px}.forecast-day{padding:12px 6px}}");
    s.append("@media(max-width:380px){.weather-temp{font-size:4.5rem}.forecast{grid-template-columns:repeat(3,1fr)}}");
    s
}

// ============================================================================
// HTML: ERROR
// ============================================================================

public func errorHtml(msg: String) -> String {
    var t = Template();
    t.put("msg", msg);
    t.render("<div class=\"error\">{msg}</div>")
}
