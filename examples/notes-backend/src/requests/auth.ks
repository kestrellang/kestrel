module notes.requests

import quill.value.(Value)
import quill.deserialize.(Deserialize, extractString)
import quill.error.(DeserializeError)

public struct CreateUserRequest: Deserialize, Cloneable {
    public var email: String
    public var firstName: String
    public var lastName: String
    public var password: String

    public static func fromValue(value: Value) -> Result[CreateUserRequest, DeserializeError] {
        let email = try extractString(from: value, "email");
        let firstName = try extractString(from: value, "firstName");
        let lastName = try extractString(from: value, "lastName");
        let password = try extractString(from: value, "password");
        .Ok(CreateUserRequest(email: email, firstName: firstName, lastName: lastName, password: password))
    }

    public func clone() -> CreateUserRequest {
        CreateUserRequest(
            email: self.email.clone(),
            firstName: self.firstName.clone(),
            lastName: self.lastName.clone(),
            password: self.password.clone()
        )
    }
}

public struct LoginRequest: Deserialize, Cloneable {
    public var email: String
    public var password: String

    public static func fromValue(value: Value) -> Result[LoginRequest, DeserializeError] {
        let email = try extractString(from: value, "email");
        let password = try extractString(from: value, "password");
        .Ok(LoginRequest(email: email, password: password))
    }

    public func clone() -> LoginRequest {
        LoginRequest(
            email: self.email.clone(),
            password: self.password.clone()
        )
    }
}
