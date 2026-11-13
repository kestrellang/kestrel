// Kestrel Pokédex
//
// A Kanto-region pokedex (Gen 1, 151 entries). The grid is rendered from a
// hardcoded list; the detail page fetches live data from PokéAPI.

module pokedex.main

import perch.(App, Logger, Get, HttpError, Query)
import http.content.(Html)
import swoop.swoop.(Swoop)
import quill.value.(Value)
import quill.deserialize.(Deserialize)
import quill.error.(DeserializeError)
import quill.json.parser.(parseJson)
import pokedex.ui.(landingPageHtml, gridItemsHtml, detailPageHtml, errorPageHtml, filterKanto)
import http.url.(percentDecode)

// ============================================================================
// CONTEXT
// ============================================================================

struct Ctx: Cloneable {
    var pokeApiBase: String
    var landingHtml: String

    func clone() -> Ctx {
        Ctx(pokeApiBase: self.pokeApiBase.clone(), landingHtml: self.landingHtml.clone())
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

/// `/search?q=<name or number>&type=<type>`: both optional, percent-decoded.
struct SearchQuery: Deserialize {
    var text: String
    var typeFilter: String

    static func fromValue(value: Value) -> SearchQuery throws DeserializeError {
        let text = percentDecode(queryParam(value, "q") ?? "");
        let typeFilter = percentDecode(queryParam(value, "type") ?? "");
        SearchQuery(text: text, typeFilter: typeFilter)
    }
}

/// `/pokemon?id=<n>`. Kept raw: a missing and an unparsable id get
/// different error pages.
struct PokemonQuery: Deserialize {
    var id: String?

    static func fromValue(value: Value) -> PokemonQuery throws DeserializeError =
        PokemonQuery(id: queryParam(value, "id"))
}

struct Handlers {
    static func index(ctx: Ctx, input: ()) -> Html throws HttpError = Html(ctx.landingHtml.clone())

    /// The grid items matching the search box and type filter (an htmx fragment).
    static func search(ctx: Ctx, Query { value: query }: Query[SearchQuery]) -> Html throws HttpError =
        Html(gridItemsHtml(filterKanto(query.text, query.typeFilter)))

    /// The detail page, fetched live from PokéAPI. Bad ids and fetch
    /// failures are error pages, not HTTP errors.
    static func pokemon(ctx: Ctx, Query { value: query }: Query[PokemonQuery]) -> Html throws HttpError {
        guard let some idStr = query.id else {
            return Html(errorPageHtml("Missing pokemon id."))
        }
        guard let some id = Int64(parsing: idStr) else {
            return Html(errorPageHtml("Invalid pokemon id."))
        }
        if id < 1 or id > 151 {
            return Html(errorPageHtml("That pokemon isn't in the Kanto pokedex."))
        };

        var url = String();
        url.append(ctx.pokeApiBase);
        url.append("/api/v2/pokemon/");
        url.append(id.formatted());

        let res = match Swoop().fetch(url) {
            .Ok(res) => res,
            .Err(_) => return Html(errorPageHtml("Could not reach PokéAPI."))
        };
        match parseJson(res.body) {
            .Ok(json) => Html(detailPageHtml(json, id)),
            .Err(_) => Html(errorPageHtml("Failed to parse PokéAPI response."))
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

@main
func main() {
    let landing = landingPageHtml();
    let ctx = Ctx(pokeApiBase: "https://pokeapi.co", landingHtml: landing);
    let app = App(ctx).using(Logger());
    app.on(Get("/"), Handlers.index);
    app.on(Get("/search"), Handlers.search);
    app.on(Get("/pokemon"), Handlers.pokemon);

    let port: UInt16 = 8080;
    println("Starting pokedex on http://localhost:8080");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            println("Error: " + e.description());
        }
    }
}
