// All of these should have type inference between the type and the initial value, or getter and setters

public struct Foo {
  // Immutable field
  public let structLet: Int64 = 0;

  // Mutable field
  public var structVar: Int64 = 0;

  // Imutable global variable, assigned in global init (property init or something)
  public static let structStaticLet: Int64 = 0;

  // Mutable global variable, assigned in global init (property init or something)
  public static var structStaticVar: Int64 = 0;

  // Not allowed, should error that computed properties must be declared with var
  public let structComputedLet: Int64 { 0 }

  // Acts as a computed properties, allows getter and setter
  public var structComputedVar: Int64 { 0 }

  // Not allowed, should error that computed properties must be declared with var
  public static let structStaticComputedLet: Int64 { 0 }

  // Computed property that doesn't take self
  public static var structStaticComputedVar: Int64 { 0 }
}

func main() {
  let foo = Foo(structLet: 0, structVar: 0);
}