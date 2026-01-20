func main() {
  println(intToString(1));
  println(intToString(10));
}

func intToString(n: Int64) -> String {
    if n == 0 {
        return "0"
    }

    var num = n;
    var negative = false;
    if num < 0 {
        negative = true;
        num = 0 - num;
    }

    var result = "";
    while num > 0 {
        let digit = num % 10;
        let ch = match digit {
            0 => "0",
            1 => "1",
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            _ => "9",
        };
        result = ch + result;
        num = num / 10;
    }

    if negative {
        result = "-" + result;
    }

    result
}
