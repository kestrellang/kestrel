/// Request extractors: the `FromRequest` protocol a handler's input conforms
/// to, and the built-in extractors `Json[T]`, `Path[T]`, and `Query[T]`.

module perch

import quill.value.(Value)
import quill.serialize.(Serialize)
import quill.deserialize.(Deserialize)
import quill.json.parser.(parseJson)
import quill.json.emitter.(emitJson)
import http.status.(StatusCode)

/// A value a handler takes as input, built from the request before the
/// handler runs. A failed extraction is the response — the handler never
/// sees a malformed request.
///
/// Conformers: `Json[T]`, `Path[T]`, `Query[T]`, `()` (the handler needs
/// nothing), and tuples of extractors (`(Path[Int64], Json[Edit])`).
public protocol FromRequest {
    static func fromRequest(request: Request) -> Self throws HttpError
}

extend (): FromRequest {
    public static func fromRequest(request: Request) -> () throws HttpError = ()
}

extend (A, B): FromRequest where A: FromRequest, B: FromRequest {
    public static func fromRequest(request: Request) -> (A, B) throws HttpError {
        let a = try A.fromRequest(request);
        let b = try B.fromRequest(request);
        (a, b)
    }
}

extend (A, B, C): FromRequest where A: FromRequest, B: FromRequest, C: FromRequest {
    public static func fromRequest(request: Request) -> (A, B, C) throws HttpError {
        let a = try A.fromRequest(request);
        let b = try B.fromRequest(request);
        let c = try C.fromRequest(request);
        (a, b, c)
    }
}

extend (A, B, C, D): FromRequest where A: FromRequest, B: FromRequest, C: FromRequest, D: FromRequest {
    public static func fromRequest(request: Request) -> (A, B, C, D) throws HttpError {
        let a = try A.fromRequest(request);
        let b = try B.fromRequest(request);
        let c = try C.fromRequest(request);
        let d = try D.fromRequest(request);
        (a, b, c, d)
    }
}

// ============================================================================
// JSON
// ============================================================================

/// A JSON body: decoded from the request when `T: Deserialize`, encoded as
/// a 200 `application/json` response when `T: Serialize`.
///
/// ```
/// static func create(ctx: Ctx, body: Json[NewNote]) -> Json[Note] throws HttpError {
///     let Json { value: new } = body;
///     ...
/// }
/// ```
public struct Json[T] {
    public var value: T

    public init(value: T) {
        self.value = value
    }
}

/// Invalid JSON is a 400; JSON of the wrong shape is a 422.
extend Json[T]: FromRequest where T: Deserialize {
    public static func fromRequest(request: Request) -> Json[T] throws HttpError {
        let parsed = match parseJson(request.body) {
            .Ok(v) => v,
            .Err(e) => throw HttpError.badRequest("Invalid JSON: \(e.description())")
        };
        match T.fromValue(parsed) {
            .Ok(value) => Json(value),
            .Err(e) => throw HttpError.unprocessable(e.description())
        }
    }
}

/// A value that can't be serialized is a 500: it's a server bug, not the client's.
extend Json[T]: IntoResponse where T: Serialize {
    public consuming func intoResponse() -> Response {
        let encoded = match self.value.toValue() {
            .Ok(v) => emitJson(v),
            .Err(_) => return HttpError.serverError().intoResponse()
        };
        var response = Response.text(StatusCode.ok(), encoded);
        response.headers.setValue("Content-Type", "application/json");
        response
    }
}

// ============================================================================
// PATH
// ============================================================================

/// A type parsed from one path segment (`:id` in `/notes/:id`).
public protocol PathParameter {
    static func fromPathSegment(segment: String) -> Self?
}

extend String: PathParameter {
    public static func fromPathSegment(segment: String) -> String? = .Some(segment)
}

extend Int64: PathParameter {
    public static func fromPathSegment(segment: String) -> Int64? = Int64(parsing: segment)
}

extend Int32: PathParameter {
    public static func fromPathSegment(segment: String) -> Int32? = Int32(parsing: segment)
}

extend UInt64: PathParameter {
    public static func fromPathSegment(segment: String) -> UInt64? = UInt64(parsing: segment)
}

/// The route's path parameter, parsed. A segment that doesn't parse as `T`
/// is a 404: `/notes/abc` names no note.
///
/// `Path[T]` reads the route's FIRST `:param`. For a route with several,
/// take a tuple: `Path[(Int64, String)]`.
public struct Path[T] {
    public var value: T

    public init(value: T) {
        self.value = value
    }
}

extend Path[T]: FromRequest where T: PathParameter {
    public static func fromRequest(request: Request) -> Path[T] throws HttpError {
        guard let .Some(segment) = request.pathValues.first else {
            throw HttpError.serverError("Route has no path parameter")
        }
        guard let .Some(value) = T.fromPathSegment(segment) else {
            throw HttpError.notFound()
        }
        Path(value)
    }
}

extend Path[(A, B)]: FromRequest where A: PathParameter, B: PathParameter {
    public static func fromRequest(request: Request) -> Path[(A, B)] throws HttpError {
        guard request.pathValues.count >= 2 else {
            throw HttpError.serverError("Route has fewer than 2 path parameters")
        }
        guard let .Some(a) = A.fromPathSegment(request.pathValues(0)) else { throw HttpError.notFound() }
        guard let .Some(b) = B.fromPathSegment(request.pathValues(1)) else { throw HttpError.notFound() }
        Path((a, b))
    }
}

// ============================================================================
// QUERY
// ============================================================================

/// The query string, decoded as `T`. Each parameter arrives as a `.Str`
/// value in an object (`?page=2` → `{"page": "2"}`), so `T.fromValue`
/// parses numbers itself. A decode failure is a 400.
public struct Query[T] {
    public var value: T

    public init(value: T) {
        self.value = value
    }
}

extend Query[T]: FromRequest where T: Deserialize {
    public static func fromRequest(request: Request) -> Query[T] throws HttpError {
        var object: Dictionary[String, Value] = [:];
        for (key, value) in request.queryParams {
            object.insert(key.clone(), .Str(value.clone()));
        }
        match T.fromValue(.Obj(object)) {
            .Ok(value) => Query(value),
            .Err(e) => throw HttpError.badRequest(e.description())
        }
    }
}
