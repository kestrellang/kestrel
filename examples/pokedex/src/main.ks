// Kestrel Pokédex
//
// A Kanto-region pokedex (Gen 1, 151 entries). The grid is rendered from a
// hardcoded list; the detail page fetches live data from PokéAPI.

module pokedex.main

import perch.app.(App)
import perch.request.(Request)
import perch.response.(Response)
import perch.middleware.(logger)
import swoop.swoop.(Swoop)
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

func handleSearch(req: Request, ctx: Ctx) -> Response {
    let q = match req.query("q") {
        .Some(v) => percentDecode(v),
        .None => ""
    };
    let typeFilter = match req.query("type") {
        .Some(v) => percentDecode(v),
        .None => ""
    };
    let entries = filterKanto(q, typeFilter);
    Response.ok(html: gridItemsHtml(entries))
}

func handlePokemon(req: Request, ctx: Ctx) -> Response {
    let idStr = match req.query("id") {
        .Some(v) => v,
        .None => return Response.ok(html: errorPageHtml("Missing pokemon id."))
    };
    let id = match Int64(parsing: idStr) {
        .Some(n) => n,
        .None => return Response.ok(html: errorPageHtml("Invalid pokemon id."))
    };
    if id < 1 or id > 151 {
        return Response.ok(html: errorPageHtml("That pokemon isn't in the Kanto pokedex."))
    };

    var url = String();
    url.append(ctx.pokeApiBase);
    url.append("/api/v2/pokemon/");
    url.append(id.formatted());

    match Swoop().fetch(url) {
        .Ok(res) => {
            match parseJson(res.body) {
                .Ok(json) => Response.ok(html: detailPageHtml(json, id)),
                .Err(e) => Response.ok(html: errorPageHtml("Failed to parse PokéAPI response."))
            }
        },
        .Err(e) => {
            Response.ok(html: errorPageHtml("Could not reach PokéAPI."))
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

func main() {
    let landing = landingPageHtml();
    let ctx = Ctx(pokeApiBase: "https://pokeapi.co", landingHtml: landing);
    var app = App[Ctx](ctx);
    app.use(logger[Ctx]());

    app.onGet("/", { (req: Request, ctx: Ctx) in
        Response.ok(html: ctx.landingHtml)
    });

    app.onGet("/search", { (req: Request, ctx: Ctx) in
        handleSearch(req, ctx)
    });

    app.onGet("/pokemon", { (req: Request, ctx: Ctx) in
        handlePokemon(req, ctx)
    });

    let port: UInt16 = 8080;
    let _ = println("Starting pokedex on http://localhost:8080");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            let _ = println("Error: " + e.description());
        }
    }
}
