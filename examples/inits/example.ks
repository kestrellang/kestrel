struct Int8 {}

struct Int16 {}

struct Int32: Convertible[Int8], Convertible[Int16] {
  init (from other: Int8) {}
  init (from other: Int16) {}
}

struct Int64 {
  init (from other: Int8) {}
  init (from other: Int16) {}
}

protocol Convertible[T] {
  init (from other: T)
}