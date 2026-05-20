module notes.handlers

import perch.request.(Request)
import perch.response.(Response)
import perch.json_body.(JsonBody)
import quill.value.(Value)
import notes.context.(AppCtx)
import notes.helpers.(errorJson, parseBody, currentTimestamp)
import notes.requests.(CreateUserRequest, LoginRequest)
import notes.db.(findUserByEmail, findPasswordByEmail, createUser, createToken)
import notes.crypto.(hashPassword, generateSalt, generateToken)

public func handleRegister(req: Request, ctx: AppCtx) -> Response {
    let body = match parseBody[CreateUserRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    let db = ctx.db;

    guard let .Ok(existingUser) = findUserByEmail(db, body.email) else {
        return Response.internalServerError()
    }
    if let .Some(_) = existingUser {
        return Response.conflict(JsonBody(fromRaw: errorJson("Email already registered")))
    };

    let salt = generateSalt(body.email);
    let passwordHash = hashPassword(body.password, salt);
    let now = currentTimestamp();

    guard let .Ok(user) = createUser(db, body.firstName, body.lastName, body.email, salt, passwordHash, now) else {
        return Response.internalServerError()
    }
    guard let .Ok(json) = JsonBody(user) else { return Response.internalServerError() }
    Response.created(json)
}

public func handleLogin(req: Request, ctx: AppCtx) -> Response {
    let body = match parseBody[LoginRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    let db = ctx.db;

    guard let .Ok(some passwordRow) = findPasswordByEmail(db, body.email) else {
        return Response.unauthorized()
    }

    let hash = hashPassword(body.password, passwordRow.salt);
    guard hash == passwordRow.passwordHash else {
        return Response.unauthorized()
    }

    let token = generateToken(passwordRow.id, passwordRow.salt);
    let now = currentTimestamp();

    guard let .Ok(_) = createToken(db, passwordRow.id, token.clone(), now) else {
        return Response.internalServerError()
    }

    var obj = Dictionary[String, Value]();
    obj.insert("token", Value.Str(token));
    Response.ok(JsonBody(fromRaw: Value.Obj(obj)))
}
