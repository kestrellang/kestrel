module notes.models

import quill.value.(Value)
import quill.serialize.(Serialize)
import quill.error.(SerializeError)

// Response body of POST /auth/login: `{"token": "<bearer token>"}`.
public struct TokenResponse: Serialize, Cloneable {
    public var token: String

    public func toValue() -> Result[Value, SerializeError] {
        var obj = Dictionary[String, Value]();
        obj.insert("token", Value.Str(self.token.clone()));
        .Ok(Value.Obj(obj))
    }

    public func clone() -> TokenResponse {
        TokenResponse(token: self.token.clone())
    }
}
