// Weather Dashboard
//
// A weather dashboard that wraps the Open-Meteo API using htmx.

module weather.main

import perch.(App, Logger, Get, HttpError, Query)
import http.content.(Html)
import swoop.swoop.(Swoop)
import quill.value.(Value)
import quill.deserialize.(Deserialize)
import quill.error.(DeserializeError)
import quill.json.parser.(parseJson)
import weather.ui.(pageHtml, searchResultsHtml, weatherPageHtml, errorHtml)
import http.url.(percentEncode, percentDecode)

// ============================================================================
// CONTEXT
// ============================================================================

struct Ctx: Cloneable {
    var geoBase: String
    var weatherBase: String
    var landingHtml: String

    func clone() -> Ctx {
        Ctx(geoBase: self.geoBase.clone(), weatherBase: self.weatherBase.clone(), landingHtml: self.landingHtml.clone())
    }
}

// ============================================================================
// ROUTES
// ============================================================================

/// The query parameter `key` of a decoded query string, or `.None`.
func queryParam(value: Value, key: String) -> String? {
    guard let some .Str(raw) = value.value(for: key) else { return .None }
    .Some(raw)
}

/// `/search?q=<city>`, percent-decoded; "" when absent.
struct SearchQuery: Deserialize {
    var city: String

    static func fromValue(value: Value) -> SearchQuery throws DeserializeError =
        SearchQuery(city: percentDecode(queryParam(value, "q") ?? ""))
}

/// `/weather?lat=&lon=&name=`. The coordinates stay optional so a missing
/// one is an error fragment naming it, not a bare 400.
struct WeatherQuery: Deserialize {
    var lat: String?
    var lon: String?
    var name: String

    static func fromValue(value: Value) -> WeatherQuery throws DeserializeError {
        let lat = queryParam(value, "lat");
        let lon = queryParam(value, "lon");
        let name = match queryParam(value, "name") {
            .Some(v) => percentDecode(v),
            .None => "Unknown"
        };
        WeatherQuery(lat: lat, lon: lon, name: name)
    }
}

struct Handlers {
    static func index(ctx: Ctx, input: ()) -> Html throws HttpError = Html(ctx.landingHtml.clone())

    /// Geocoding matches for the search box (an htmx fragment). Fetch
    /// failures render as an error fragment.
    static func search(ctx: Ctx, Query { value: query }: Query[SearchQuery]) -> Html throws HttpError {
        if query.city.bytes.count == 0 {
            return Html("")
        };

        let url = ctx.geoBase + "/v1/search?name=" + percentEncode(query.city) + "&count=5&language=en";

        let res = match Swoop().fetch(url) {
            .Ok(res) => res,
            .Err(_) => return Html(errorHtml("Could not reach weather service."))
        };
        match parseJson(res.body) {
            .Ok(json) => Html(searchResultsHtml(json)),
            .Err(_) => Html(errorHtml("Failed to parse response."))
        }
    }

    /// The forecast page for a location.
    static func weather(ctx: Ctx, Query { value: query }: Query[WeatherQuery]) -> Html throws HttpError {
        guard let some lat = query.lat else {
            return Html(errorHtml("Missing latitude."))
        }
        guard let some lon = query.lon else {
            return Html(errorHtml("Missing longitude."))
        }

        var url = String();
        url.append(ctx.weatherBase);
        url.append("/v1/forecast?latitude=");
        url.append(lat);
        url.append("&longitude=");
        url.append(lon);
        url.append(##"&current=temperature_2m,weather_code,wind_speed_10m,relative_humidity_2m,apparent_temperature,surface_pressure,uv_index&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,sunrise,sunset&hourly=temperature_2m,weather_code&temperature_unit=fahrenheit&wind_speed_unit=mph&precipitation_unit=inch&forecast_days=7&timezone=auto"##);

        let res = match Swoop().fetch(url) {
            .Ok(res) => res,
            .Err(_) => return Html(errorHtml("Could not reach weather service."))
        };
        match parseJson(res.body) {
            .Ok(json) => Html(weatherPageHtml(json, query.name)),
            .Err(_) => Html(errorHtml("Failed to parse weather data."))
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

@main
func main() {
    let landing = pageHtml();
    let ctx = Ctx(geoBase: "https://geocoding-api.open-meteo.com", weatherBase: "https://api.open-meteo.com", landingHtml: landing);
    let app = App(ctx).using(Logger());
    app.on(Get("/"), Handlers.index);
    app.on(Get("/search"), Handlers.search);
    app.on(Get("/weather"), Handlers.weather);

    let port: UInt16 = 8080;
    println("Starting weather dashboard on http://localhost:8080");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            println("Error: " + e.description());
        }
    }
}
