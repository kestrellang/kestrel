// Weather Dashboard
//
// A weather dashboard that wraps the Open-Meteo API using htmx.
// Uses socat proxies for HTTP→HTTPS bridging:
//   socat TCP-LISTEN:3002,reuseaddr,fork OPENSSL:api.open-meteo.com:443 &
//   socat TCP-LISTEN:3001,reuseaddr,fork OPENSSL:geocoding-api.open-meteo.com:443 &

module weather.main

import perch.app.(App)
import perch.request.(Request)
import perch.response.(Response)
import swoop.swoop.(Swoop)
import quill.json.parser.(parseJson)
import weather.ui.(pageHtml, searchResultsHtml, weatherPageHtml, errorHtml)
import weather.util.(urlEncode, urlDecode)
import perch.middleware.(logger)

// ============================================================================
// CONTEXT
// ============================================================================

struct Ctx: Cloneable {
    var geoBase: String
    var weatherBase: String

    func clone() -> Ctx {
        Ctx(geoBase: self.geoBase.clone(), weatherBase: self.weatherBase.clone())
    }
}

// ============================================================================
// ROUTES
// ============================================================================

func handleSearch(req: Request, ctx: Ctx) -> Response {
    let city = match req.query("q") {
        .Some(v) => urlDecode(v),
        .None => ""
    };
    if city.byteCount == 0 {
        return Response.ok(html: "")
    };

    let url = ctx.geoBase + "/v1/search?name=" + urlEncode(city) + "&count=5&language=en";

    match Swoop().fetch(url) {
        .Ok(res) => {
            match parseJson(res.body) {
                .Ok(json) => Response.ok(html: searchResultsHtml(json)),
                .Err(e) => Response.ok(html: errorHtml("Failed to parse response."))
            }
        },
        .Err(e) => {
            Response.ok(html: errorHtml("Could not reach weather service."))
        }
    }
}

func handleWeather(req: Request, ctx: Ctx) -> Response {
    let lat = match req.query("lat") {
        .Some(v) => v,
        .None => return Response.ok(html: errorHtml("Missing latitude."))
    };
    let lon = match req.query("lon") {
        .Some(v) => v,
        .None => return Response.ok(html: errorHtml("Missing longitude."))
    };
    let name = match req.query("name") {
        .Some(v) => urlDecode(v),
        .None => "Unknown"
    };

    var url = String();
    url.append(ctx.weatherBase);
    url.append("/v1/forecast?latitude=");
    url.append(lat);
    url.append("&longitude=");
    url.append(lon);
    url.append("&current=temperature_2m,weather_code,wind_speed_10m,relative_humidity_2m");
    url.append("&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum");
    url.append("&temperature_unit=fahrenheit&wind_speed_unit=mph&precipitation_unit=inch");
    url.append("&forecast_days=7");

    match Swoop().fetch(url) {
        .Ok(res) => {
            match parseJson(res.body) {
                .Ok(json) => Response.ok(html: weatherPageHtml(json, name)),
                .Err(e) => Response.ok(html: errorHtml("Failed to parse weather data."))
            }
        },
        .Err(e) => {
            Response.ok(html: errorHtml("Could not reach weather service."))
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

func main() {
    let ctx = Ctx(geoBase: "http://localhost:3001", weatherBase: "http://localhost:3002");
    var app = App[Ctx](ctx);
    app.use(logger[Ctx]());

    app.onGet("/", { (req: Request, ctx: Ctx) in
        Response.ok(html: pageHtml())
    });

    app.onGet("/search", { (req: Request, ctx: Ctx) in
        handleSearch(req, ctx)
    });

    app.onGet("/weather", { (req: Request, ctx: Ctx) in
        handleWeather(req, ctx)
    });

    let port: UInt16 = 8080;
    let _ = println("Starting weather dashboard on http://localhost:8080");
    let _ = println("Make sure socat proxies are running:");
    let _ = println("  socat TCP-LISTEN:3002,reuseaddr,fork OPENSSL:api.open-meteo.com:443 &");
    let _ = println("  socat TCP-LISTEN:3001,reuseaddr,fork OPENSSL:geocoding-api.open-meteo.com:443 &");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            let _ = println("Error: " + e.description());
        }
    }
}
