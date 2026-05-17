// UI layer: all HTML generation

module weather.ui

import quill.value.(Value)
import plume.(Template)
import weather.data.(getFloat, getString, getInt, getField, getArrayField, getFloatFromArray, getIntFromArray, getStringFromArray, weatherEmoji, weatherDescription, weatherClass, tempColorClass, evocativeDescription, formatDateLabel, parseHourFromIso, formatHourLabel, formatSunTime, uvDescription, pressureDescription, feelsLikeNote)
import http.url.(percentEncode)
import weather.util.(formatTemp, formatTempWhole, formatInt)

// ============================================================================
// SHARED CSS (landing page + dropdown)
// ============================================================================

func baseCss() -> String {
    var s = String(capacity: 3072);
    s.append(##"*{box-sizing:border-box;margin:0;padding:0}body{font-family:'Inter',system-ui,-apple-system,sans-serif;color:#f1f0f5;min-height:100vh;-webkit-font-smoothing:antialiased}a{color:inherit;text-decoration:none}.search-wrap{position:relative}.search-wrap input{width:100%;padding:16px 20px;border-radius:16px;border:1px solid rgba(255,255,255,0.1);background:rgba(255,255,255,0.06);backdrop-filter:blur(12px);-webkit-backdrop-filter:blur(12px);color:#f1f0f5;font-size:1rem;font-family:inherit;outline:none;transition:border-color 0.3s,box-shadow 0.3s,background 0.3s}.search-wrap input::placeholder{color:#6b6784}.search-wrap input:focus{border-color:rgba(167,139,250,0.5);box-shadow:0 0 0 3px rgba(167,139,250,0.15);background:rgba(255,255,255,0.09)}.dropdown{position:absolute;left:0;right:0;top:100%;margin-top:8px;z-index:100;max-height:320px;overflow-y:auto;border-radius:16px}.dropdown:empty{display:none}.dropdown:not(:empty){background:rgba(20,20,40,0.95);backdrop-filter:blur(20px);-webkit-backdrop-filter:blur(20px);border:1px solid rgba(255,255,255,0.1);box-shadow:0 16px 48px rgba(0,0,0,0.5);padding:6px}.city-item{display:block;padding:14px 16px;border-radius:12px;cursor:pointer;transition:all 0.2s ease;animation:fadeSlideIn 0.25s ease both}.city-item:hover{background:rgba(167,139,250,0.12)}.city-name{font-weight:600;color:#f1f0f5;font-size:0.95rem}.city-detail{font-size:0.8rem;color:#8b87a0;margin-top:2px}.error{background:rgba(239,68,68,0.1);border:1px solid rgba(239,68,68,0.2);border-radius:14px;padding:16px 20px;color:#fca5a5;text-align:center;font-size:0.9rem;animation:fadeSlideIn 0.3s ease both}.htmx-indicator{display:none}.htmx-request .htmx-indicator{display:block}.search-spinner{position:absolute;right:16px;top:50%;transform:translateY(-50%);width:18px;height:18px;border:2px solid rgba(167,139,250,0.2);border-top-color:#a78bfa;border-radius:50%;animation:spin 0.7s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}@keyframes fadeSlideIn{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:translateY(0)}}@keyframes float{0%,100%{transform:translateY(0)}50%{transform:translateY(-8px)}}"##);
    s
}

// ============================================================================
// LANDING PAGE CSS
// ============================================================================

func landingCss() -> String {
    var s = String(capacity: 2048);

    // Background and ambient orbs
    s.append(##"body{background:#08081a;overflow-x:hidden}.amb{position:fixed;inset:0;z-index:0;overflow:hidden;pointer-events:none}.orb{position:absolute;border-radius:50%;filter:blur(90px);opacity:0.35;will-change:transform}.orb-1{width:520px;height:520px;background:radial-gradient(circle,#2563eb,transparent 70%);top:-12%;left:-8%;animation:drift1 22s ease-in-out infinite alternate}.orb-2{width:420px;height:420px;background:radial-gradient(circle,#7c3aed,transparent 70%);bottom:-14%;right:-8%;animation:drift2 26s ease-in-out infinite alternate}.orb-3{width:320px;height:320px;background:radial-gradient(circle,#0ea5e9,transparent 70%);top:38%;right:25%;animation:drift3 19s ease-in-out infinite alternate}"##);

    // Layout
    s.append(##".landing{position:relative;z-index:1;display:flex;flex-direction:column;align-items:center;justify-content:center;min-height:100vh;padding:40px 24px}"##);

    // Hero
    s.append(##".hero{text-align:center;margin-bottom:52px}.hero-icon{font-size:3.8rem;margin-bottom:20px;opacity:0;animation:rise 1s cubic-bezier(0.16,1,0.3,1) 0.2s forwards}.hero h1{font-family:'Fraunces',Georgia,serif;font-size:clamp(3rem,8vw,5.2rem);font-weight:300;letter-spacing:-0.03em;line-height:1.05;color:#fff;opacity:0;animation:rise 1.2s cubic-bezier(0.16,1,0.3,1) 0.35s forwards}.sub{font-size:1.05rem;color:rgba(255,255,255,0.38);margin-top:20px;font-weight:400;letter-spacing:0.03em;opacity:0;animation:rise 1s cubic-bezier(0.16,1,0.3,1) 0.55s forwards}"##);

    // Search overrides for landing
    s.append(##".landing .search-wrap{width:100%;max-width:460px;opacity:0;animation:rise 1s cubic-bezier(0.16,1,0.3,1) 0.7s forwards}.landing .search-wrap input{padding:20px 24px;border-radius:20px;font-size:1.08rem;background:rgba(255,255,255,0.06);border:1px solid rgba(255,255,255,0.09)}.landing .search-wrap input:focus{border-color:rgba(99,130,255,0.45);box-shadow:0 0 0 3px rgba(99,130,255,0.12);background:rgba(255,255,255,0.09)}.landing .dropdown{border-radius:20px}"##);

    // Attribution
    s.append(##".attr{position:absolute;bottom:28px;font-size:0.6rem;letter-spacing:0.22em;text-transform:uppercase;color:rgba(255,255,255,0.16);font-weight:500;opacity:0;animation:rise 0.8s ease 1.2s forwards}"##);

    // Animations
    s.append(##"@keyframes drift1{from{transform:translate(0,0) scale(1)}to{transform:translate(80px,60px) scale(1.12)}}@keyframes drift2{from{transform:translate(0,0) scale(1)}to{transform:translate(-65px,-45px) scale(1.15)}}@keyframes drift3{from{transform:translate(0,0) scale(1)}to{transform:translate(45px,-55px) scale(0.88)}}@keyframes rise{from{opacity:0;transform:translateY(22px)}to{opacity:1;transform:translateY(0)}}"##);

    // Responsive
    s.append(##"@media(max-width:640px){.hero h1{font-size:2.8rem}.hero-icon{font-size:3rem}.sub{font-size:0.92rem}.landing .search-wrap input{padding:16px 20px;font-size:1rem}.orb-1{width:340px;height:340px}.orb-2{width:280px;height:280px}.orb-3{width:200px;height:200px}}@media(prefers-reduced-motion:reduce){.orb{animation:none!important}*{animation-duration:0.01ms!important;transition-duration:0.01ms!important}}"##);

    s
}

// ============================================================================
// HTML: LANDING PAGE
// ============================================================================

public func pageHtml() -> String {
    var h = String(capacity: 4096);
    h.append(##"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Kestrel Weather</title><link rel="preconnect" href="https://fonts.googleapis.com"><link rel="preconnect" href="https://fonts.gstatic.com" crossorigin><link href="https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,300;9..144,400&family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet"><script src="https://unpkg.com/htmx.org@1.9.10"></script><style>"##);
    h.append(baseCss());
    h.append(landingCss());
    h.append(##"</style></head><body><div class="amb"><div class="orb orb-1"></div><div class="orb orb-2"></div><div class="orb orb-3"></div></div><div class="landing"><div class="hero"><div class="hero-icon">&#x2601;&#xFE0F;</div><h1>Kestrel<br>Weather</h1><p class="sub">Your forecast, beautifully simple</p></div><div class="search-wrap"><input type="text" name="q" placeholder="Search for a city..." autocomplete="off" autofocus hx-get="/search" hx-trigger="keyup changed delay:300ms" hx-target="#dropdown"><div class="htmx-indicator"><div class="search-spinner"></div></div><div id="dropdown" class="dropdown"></div></div><div class="attr">Powered by Open-Meteo</div></div></body></html>"##);
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

                        t.setRaw("lat", lat.formatted());
                        t.setRaw("lon", lon.formatted());
                        t.put("name", name);
                        t.setRaw("encodedName", percentEncode(name));
                        t.setInt("delay", i * 40);

                        var detail = String();
                        if admin1.byteCount > 0 {
                            detail.append(admin1);
                            detail.append(", ")
                        };
                        detail.append(country);
                        t.put("detail", detail);

                        h.append(t.render(##"<a class="city-item" style="animation-delay:{delay}ms" href="/weather?lat={lat}&lon={lon}&name={encodedName}"><div class="city-name">{name}</div><div class="city-detail">{detail}</div></a>"##));
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
// HTML: WEATHER PAGE
// ============================================================================

public func weatherPageHtml(json: Value, cityName: String) -> String {
    var h = String(capacity: 8192);
    var t = Template();

    let current = getField(json, "current");
    let daily = getField(json, "daily");
    let hourly = getField(json, "hourly");

    let temp = getFloat(getField(current, "temperature_2m"));
    let code = getInt(getField(current, "weather_code"));
    let wind = getFloat(getField(current, "wind_speed_10m"));
    let humidity = getFloat(getField(current, "relative_humidity_2m"));
    let feelsLike = getFloat(getField(current, "apparent_temperature"));
    let pressure = getFloat(getField(current, "surface_pressure"));
    let uv = getFloat(getField(current, "uv_index"));
    let currentTime = getString(getField(current, "time"));
    let currentHour = parseHourFromIso(currentTime);

    let humInt = match humidity.toInt64() {
        .Some(n) => n,
        .None => 0
    };
    let uvInt = match uv.toInt64() {
        .Some(n) => n,
        .None => 0
    };
    let pressureInt = match pressure.toInt64() {
        .Some(n) => n,
        .None => 0
    };

    let wClass = weatherClass(code);

    // Today's high/low from daily arrays
    let dailyHighs = getArrayField(daily, "temperature_2m_max");
    let dailyLows = getArrayField(daily, "temperature_2m_min");
    let todayHigh = getFloatFromArray(dailyHighs, 0);
    let todayLow = getFloatFromArray(dailyLows, 0);

    // Sunrise/sunset
    let sunrises = getArrayField(daily, "sunrise");
    let sunsets = getArrayField(daily, "sunset");
    let sunriseStr = formatSunTime(getStringFromArray(sunrises, 0));
    let sunsetStr = formatSunTime(getStringFromArray(sunsets, 0));

    // -- Head --
    h.append(##"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">"##);
    t.put("city", cityName);
    h.append(t.render(##"<title>{city} &mdash; Kestrel Weather</title>"##));
    h.append(##"<link rel="preconnect" href="https://fonts.googleapis.com"><link href="https://fonts.googleapis.com/css2?family=Inter:wght@200;300;400;500;600;700&display=swap" rel="stylesheet"><script src="https://unpkg.com/htmx.org@1.9.10"></script><style>"##);
    h.append(baseCss());
    h.append(weatherPageCss());
    h.append("</style></head>");

    // -- Body --
    t.setRaw("wClass", wClass);
    h.append(t.render(##"<body class="{wClass}">"##));

    // Header
    h.append(##"<header class="hdr"><a href="/" class="logo">&#x2601; Kestrel Weather</a><div class="search-wrap hdr-search"><input type="text" name="q" placeholder="Search city..." autocomplete="off" hx-get="/search" hx-trigger="keyup changed delay:300ms" hx-target="#hdr-dd"><div class="htmx-indicator"><div class="search-spinner"></div></div><div id="hdr-dd" class="dropdown"></div></div></header>"##);

    // Hero
    t.put("city", cityName);
    t.setRaw("emoji", weatherEmoji(code));
    t.setRaw("temp", formatTempWhole(temp));
    t.setRaw("desc", evocativeDescription(code));
    t.setRaw("hi", formatTempWhole(todayHigh));
    t.setRaw("lo", formatTempWhole(todayLow));
    h.append(t.render(##"<main class="wx"><section class="hero"><div class="hero-icon">{emoji}</div><div class="hero-temp">{temp}<span class="deg">&deg;</span></div><div class="hero-city">{city}</div><div class="hero-cond">{desc}</div><div class="hero-hilo">H:{hi}&deg;  L:{lo}&deg;</div></section>"##));

    // Hourly forecast
    h.append(##"<section class="card anim" style="animation-delay:0.1s"><div class="card-hd">&#x1F552; Hourly Forecast</div><div class="card-sep"></div><div class="hourly">"##);

    let hourlyTimes = getArrayField(hourly, "time");
    let hourlyTemps = getArrayField(hourly, "temperature_2m");
    let hourlyCodes = getArrayField(hourly, "weather_code");

    var hi: Int64 = 0;
    while hi < 24 {
        let hourIdx = currentHour + hi;
        if hourIdx >= hourlyTimes.count { break };

        let hTimeStr = getStringFromArray(hourlyTimes, hourIdx);
        let hTemp = getFloatFromArray(hourlyTemps, hourIdx);
        let hCode = getIntFromArray(hourlyCodes, hourIdx);

        t.setRaw("hlabel", formatHourLabel(hTimeStr, hi));
        t.setRaw("hicon", weatherEmoji(hCode));
        t.setRaw("htemp", formatTempWhole(hTemp));

        h.append(t.render(##"<div class="hr"><div class="hr-t">{hlabel}</div><div class="hr-i">{hicon}</div><div class="hr-v">{htemp}&deg;</div></div>"##));
        hi = hi + 1
    };

    h.append("</div></section>");

    // 7-day forecast — compute global min/max for bar scaling
    match daily.value(forKey: "time") {
        .Some(timesVal) => {
            match timesVal.asArray() {
                .Some(times) => {
                    let codes = getArrayField(daily, "weather_code");
                    let rain = getArrayField(daily, "precipitation_sum");

                    var globalMin: Float64 = 200.0;
                    var globalMax: Float64 = -200.0;
                    var di: Int64 = 0;
                    while di < times.count {
                        let dh = getFloatFromArray(dailyHighs, di);
                        let dl = getFloatFromArray(dailyLows, di);
                        if dl < globalMin { globalMin = dl };
                        if dh > globalMax { globalMax = dh };
                        di = di + 1
                    };
                    let range = globalMax - globalMin;

                    h.append(##"<section class="card anim" style="animation-delay:0.2s"><div class="card-hd">&#x1F4C5; 7-Day Forecast</div><div class="card-sep"></div>"##);

                    var i: Int64 = 0;
                    while i < times.count {
                        let dateStr = getString(times(unchecked: i));
                        let dayCode = getIntFromArray(codes, i);
                        let high = getFloatFromArray(dailyHighs, i);
                        let low = getFloatFromArray(dailyLows, i);
                        let precip = getFloatFromArray(rain, i);

                        let barLeftF = if range > 0.0 { (low - globalMin) / range * 100.0 } else { 0.0 };
                        let barWidthF = if range > 0.0 { (high - low) / range * 100.0 } else { 100.0 };
                        let barLeft = match barLeftF.toInt64() {
                            .Some(n) => n,
                            .None => 0
                        };
                        let barWidth = match barWidthF.toInt64() {
                            .Some(n) => if n < 5 { 5 } else { n },
                            .None => 100
                        };

                        t.setRaw("dLabel", formatDateLabel(dateStr, i));
                        t.setRaw("dIcon", weatherEmoji(dayCode));
                        t.setRaw("dLo", formatTempWhole(low));
                        t.setRaw("dHi", formatTempWhole(high));
                        t.setInt("bL", barLeft);
                        t.setInt("bW", barWidth);
                        t.setRaw("tClass", tempColorClass(high));

                        var precipTag = String();
                        if precip > 0.01 {
                            t.setRaw("pp", formatTemp(precip));
                            precipTag = t.render(##"<span class="dy-rain">&#x1F4A7; {pp}"</span>"##)
                        };
                        t.setRaw("precipTag", precipTag);

                        h.append(t.render(##"<div class="dy"><span class="dy-d">{dLabel}</span><span class="dy-i">{dIcon}</span>{precipTag}<span class="dy-lo">{dLo}&deg;</span><span class="dy-bar"><span class="dy-fill" style="left:{bL}%;width:{bW}%"></span></span><span class="dy-hi {tClass}">{dHi}&deg;</span></div>"##));
                        i = i + 1
                    };

                    h.append("</section>")
                },
                .None => {}
            }
        },
        .None => {}
    };

    // Detail cards
    t.setRaw("fl", formatTempWhole(feelsLike));
    t.setRaw("flNote", feelsLikeNote(feelsLike, temp));
    t.setInt("uvVal", uvInt);
    t.setRaw("uvNote", uvDescription(uvInt));
    t.setInt("humVal", humInt);
    t.setRaw("windVal", formatTemp(wind));
    t.setInt("pressVal", pressureInt);
    t.setRaw("pressNote", pressureDescription(pressure));
    t.setRaw("rise", sunriseStr);
    t.setRaw("set", sunsetStr);

    h.append(t.render(##"<section class="dg anim" style="animation-delay:0.3s"><div class="dc"><div class="dc-l">&#x1F321;&#xFE0F; Feels Like</div><div class="dc-v">{fl}&deg;</div><div class="dc-n">{flNote}</div></div><div class="dc"><div class="dc-l">&#x2600;&#xFE0F; UV Index</div><div class="dc-v">{uvVal}</div><div class="dc-n">{uvNote}</div></div><div class="dc"><div class="dc-l">&#x1F4A7; Humidity</div><div class="dc-v">{humVal}<span class="dc-u">%</span></div><div class="dc-n">The dew point is {fl}&deg;</div></div><div class="dc"><div class="dc-l">&#x1F4A8; Wind</div><div class="dc-v">{windVal}<span class="dc-u"> mph</span></div><div class="dc-n">Current wind speed</div></div><div class="dc"><div class="dc-l">&#x23F1;&#xFE0F; Pressure</div><div class="dc-v">{pressVal}<span class="dc-u"> hPa</span></div><div class="dc-n">{pressNote}</div></div><div class="dc"><div class="dc-l">&#x1F305; Sunrise &amp; Sunset</div><div class="dc-v">&#x2191; {rise}</div><div class="dc-n">&#x2193; {set}</div></div></section>"##));

    h.append("</main></body></html>");
    h
}

// ============================================================================
// WEATHER PAGE CSS
// ============================================================================

func weatherPageCss() -> String {
    var s = String(capacity: 4096);

    // Backgrounds per weather condition
    s.append(##"body.weather-sunny{background:linear-gradient(170deg,#0c1a3a 0%,#1a3060 35%,#2e4a7a 70%,#4a6a9a 100%)}body.weather-cloudy{background:linear-gradient(170deg,#111118 0%,#1a1a28 40%,#252535 100%)}body.weather-rainy{background:linear-gradient(170deg,#0a1520 0%,#0f2030 40%,#162838 100%)}body.weather-snowy{background:linear-gradient(170deg,#101520 0%,#182030 40%,#202838 100%)}body.weather-stormy{background:linear-gradient(170deg,#100818 0%,#1a1028 40%,#241838 100%)}body.weather-foggy{background:linear-gradient(170deg,#111114 0%,#18181e 40%,#1e1e24 100%)}"##);

    // Header
    s.append(##".hdr{display:flex;align-items:center;justify-content:space-between;padding:14px 24px;background:rgba(0,0,0,0.15);backdrop-filter:blur(20px);-webkit-backdrop-filter:blur(20px);border-bottom:1px solid rgba(255,255,255,0.06);position:sticky;top:0;z-index:50}.logo{font-weight:600;font-size:0.85rem;color:rgba(255,255,255,0.65);letter-spacing:0.01em;transition:color 0.2s}.logo:hover{color:#fff}.hdr-search{width:240px}.hdr-search input{padding:9px 14px;border-radius:10px;font-size:0.85rem}"##);

    // Main container
    s.append(##".wx{max-width:620px;margin:0 auto;padding:20px 16px 64px}"##);

    // Hero
    s.append(##".hero{text-align:center;padding:32px 0 28px;animation:fadeSlideIn 0.6s ease both}.hero-icon{font-size:3.6rem;margin-bottom:0}.hero-temp{font-size:7rem;font-weight:200;color:#fff;line-height:1;letter-spacing:-0.06em}.deg{font-size:2.4rem;font-weight:300;color:rgba(255,255,255,0.5);vertical-align:top;margin-left:-4px}.hero-city{font-size:1.4rem;font-weight:600;color:#fff;margin-top:2px}.hero-cond{font-size:0.92rem;color:rgba(255,255,255,0.55);margin-top:2px}.hero-hilo{font-size:0.92rem;color:rgba(255,255,255,0.7);margin-top:4px;font-weight:500}"##);

    // Card
    s.append(##".card{background:rgba(255,255,255,0.05);backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px);border:1px solid rgba(255,255,255,0.08);border-radius:16px;padding:16px;margin-bottom:14px}.card-hd{font-size:0.7rem;color:rgba(255,255,255,0.45);text-transform:uppercase;letter-spacing:0.08em;font-weight:600;padding-bottom:10px}.card-sep{border-top:1px solid rgba(255,255,255,0.07);margin-bottom:12px}.anim{opacity:0;animation:fadeSlideIn 0.5s ease forwards}"##);

    // Hourly scroll
    s.append(##".hourly{display:flex;gap:0;overflow-x:auto;-webkit-overflow-scrolling:touch;scrollbar-width:none;padding-bottom:4px}.hourly::-webkit-scrollbar{display:none}.hr{flex:0 0 64px;text-align:center;padding:4px 0;transition:background 0.2s;border-radius:12px}.hr:first-child{background:rgba(255,255,255,0.06)}.hr-t{font-size:0.72rem;color:rgba(255,255,255,0.5);font-weight:500;margin-bottom:8px}.hr:first-child .hr-t{color:#fff;font-weight:600}.hr-i{font-size:1.15rem;margin-bottom:6px}.hr-v{font-size:0.88rem;font-weight:600;color:#fff}"##);

    // Daily forecast rows
    s.append(##".dy{display:flex;align-items:center;padding:11px 4px;border-bottom:1px solid rgba(255,255,255,0.05)}.dy:last-child{border-bottom:none}.dy-d{width:48px;font-size:0.82rem;color:rgba(255,255,255,0.6);font-weight:500}.dy-i{width:28px;text-align:center;font-size:1.05rem}.dy-rain{font-size:0.68rem;color:#67e8f9;width:52px;text-align:left;font-weight:500;padding-left:4px}.dy-lo{width:32px;text-align:right;font-size:0.82rem;color:rgba(255,255,255,0.4);font-weight:500}.dy-bar{flex:1;height:4px;background:rgba(255,255,255,0.08);border-radius:2px;margin:0 10px;position:relative;min-width:60px}.dy-fill{position:absolute;top:0;height:100%;border-radius:2px;background:linear-gradient(to right,#67b8f9,#8be8a0,#f9c74f,#f97171)}.dy-hi{width:32px;font-size:0.82rem;font-weight:600}"##);

    // Detail cards grid
    s.append(##".dg{display:grid;grid-template-columns:1fr 1fr;gap:12px;margin-bottom:14px}.dc{background:rgba(255,255,255,0.05);backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px);border:1px solid rgba(255,255,255,0.08);border-radius:16px;padding:16px}.dc-l{font-size:0.68rem;color:rgba(255,255,255,0.4);text-transform:uppercase;letter-spacing:0.06em;font-weight:600;margin-bottom:10px}.dc-v{font-size:2.2rem;font-weight:300;color:#fff;line-height:1.1}.dc-u{font-size:1rem;font-weight:400;color:rgba(255,255,255,0.45)}.dc-n{font-size:0.78rem;color:rgba(255,255,255,0.4);margin-top:8px;line-height:1.4}"##);

    // Temp color classes
    s.append(##".temp-freezing{color:#93c5fd}.temp-cold{color:#67e8f9}.temp-mild{color:#6ee7b7}.temp-warm{color:#fbbf24}.temp-hot{color:#f87171}"##);

    // Responsive
    s.append(##"@media(max-width:640px){.hdr{padding:10px 14px}.hdr-search{width:180px}.hdr-search input{padding:8px 12px;font-size:0.82rem}.wx{padding:12px 12px 48px}.hero-temp{font-size:5rem}.hero-icon{font-size:2.8rem}.dg{gap:8px}.dc{padding:14px}.dc-v{font-size:1.7rem}}@media(max-width:380px){.hero-temp{font-size:4rem}.dg{grid-template-columns:1fr 1fr;gap:6px}}@media(prefers-reduced-motion:reduce){*{animation-duration:0.01ms!important;transition-duration:0.01ms!important}}"##);

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
