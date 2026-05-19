module notes.handlers

import perch.request.(Request)
import perch.response.(Response)
import perch.json_body.(JsonBody)
import talon.sqlite.database.(Database)
import talon.sqlite.sql.(SQL)
import notes.context.(AppCtx)
import notes.errors.(errorJson, parseBody)
import notes.requests.(CreateUserRequest, LoginRequest)
import notes.db.(findUserByEmail, findPasswordByEmail, createUser, createToken)
import notes.middleware.(hashPassword, generateSalt, generateToken)

public func handleRegister(req: Request, ctx: AppCtx) -> Response {
    let body = match parseBody[CreateUserRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };

    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    match findUserByEmail(db, body.email) {
        .Ok(.Some(_)) => return Response.conflict(JsonBody(fromRaw: errorJson("Email already registered"))),
        .Ok(.None) => {},
        .Err(_) => return Response.internalServerError()
    };

    let salt = generateSalt(body.email);
    let passwordHash = hashPassword(body.password, salt);
    let now = "2026-01-01T00:00:00Z";

    match createUser(db, body.firstName, body.lastName, body.email, salt, passwordHash, now) {
        .Ok(user) => match JsonBody(user) {
            .Ok(json) => Response.created(json),
            .Err(_) => Response.internalServerError()
        },
        .Err(_) => Response.internalServerError()
    }
}

public func handleLogin(req: Request, ctx: AppCtx) -> Response {
    let body = match parseBody[LoginRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };

    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    let passwordRow = match findPasswordByEmail(db, body.email) {
        .Ok(.Some(row)) => row,
        .Ok(.None) => return Response.unauthorized(),
        .Err(_) => return Response.internalServerError()
    };

    let hash = hashPassword(body.password, passwordRow.salt);
    if hash != passwordRow.passwordHash {
        return Response.unauthorized()
    };

    let token = generateToken(passwordRow.id, passwordRow.salt);
    let now = "2026-01-01T00:00:00Z";

    match createToken(db, passwordRow.id, token.clone(), now) {
        .Ok(_) => {},
        .Err(_) => return Response.internalServerError()
    };

    // Return the token as a JSON object
    var obj = Dictionary[String, quill.value.Value]();
    obj.insert("token", quill.value.Value.Str(token));
    Response.ok(JsonBody(fromRaw: quill.value.Value.Obj(obj)))
}
