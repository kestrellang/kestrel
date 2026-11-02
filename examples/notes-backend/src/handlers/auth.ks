module notes.handlers

import perch.(HttpError, Json, Created)
import crypto.password.(PasswordHash)
import crypto.bytes.(Base64Style)
import crypto.random.(Token)
import datetime.(Instant)
import notes.context.(AppCtx)
import notes.requests.(CreateUserRequest, LoginRequest)
import notes.models.(User, TokenResponse)
import notes.db.(findUserByEmail, findPasswordByEmail, createUser, createToken)

// Public handlers for /auth. These run on AppCtx — there's no user yet.
public struct Auth {
    public static func register(ctx: AppCtx, Json { value: new }: Json[CreateUserRequest]) -> Created[Json[User]] throws HttpError {
        guard let null = try findUserByEmail(ctx.db, new.email) else {
            throw HttpError.conflict("Email already registered")
        }

        // Argon2id at OWASP's minimum (19 MiB, 2 passes): ~0.13s at -O1.
        let password = PasswordHash.hash(new.password);
        let user = try createUser(ctx.db, new.firstName, new.lastName, new.email, password.encoded, Instant.now().formatted());
        Created(Json(user))
    }

    public static func login(ctx: AppCtx, Json { value: credentials }: Json[LoginRequest]) -> Json[TokenResponse] throws HttpError {
        // Unknown email and wrong password are both 401, so the response
        // doesn't reveal which emails are registered.
        guard let some row = try findPasswordByEmail(ctx.db, credentials.email) else {
            throw HttpError.unauthorized()
        }
        guard let some stored = PasswordHash(encoded: row.passwordHash) else {
            throw HttpError.unauthorized()
        }
        guard stored.verify(credentials.password) else {
            throw HttpError.unauthorized()
        }

        // 256 bits of OS entropy; every login gets a fresh, unguessable token.
        let token = Token.random().base64String(.URLSafe);
        try createToken(ctx.db, row.id, token.clone(), Instant.now().formatted());
        Json(TokenResponse(token: token))
    }
}

