// test: diagnostics
// stdlib: false

module Main

enum Color {
    case Red
    case Green
    case Blue
}

func getRed() -> Color { Color.Red }
func getGreen() -> Color { Color.Green }
func getBlue() -> Color { Color.Blue }
