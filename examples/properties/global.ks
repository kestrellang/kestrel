// All of these should have type inference between the type and the initial value, or getter and setters

// Immutable global variable, assigned in global init (property init or something)
public let globalLet: Int64 = 0;

// Mutable global variable, assigned in global init (property init or something)
public var globalVar: Int64 = 0;

// Not allowed, should error that properties in global context are already static
public static let globalStaticLet: Int64 = 0;

// Not allowed, should error that properties in global context are already static
public static var globalStaticVar: Int64 = 0;

// Not allowed, should error that computed properties must be declared with var
public let globalComputedLet: Int64 { 0 }

// Acts as a computed properties, allows getter and setter, does not provide self
public var globalComputedVar: Int64 { 0 }


// Not allowed, should error that computed properties must be declared with var
public static let globalStaticComputedLet: Int64 { 0 }

// Not allowed, should error that computed properties in global context are already static
public static var globalStaticComputedVar: Int64 { 0 }