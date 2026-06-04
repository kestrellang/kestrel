// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // pow
            let two: std.numeric.Int64 = 2;
            if two.pow(10) != 1024 { return 1 }
            let three: std.numeric.Int64 = 3;
            if three.pow(4) != 81 { return 2 }
            let five: std.numeric.Int64 = 5;
            if five.pow(0) != 1 { return 3 }
            let negTwo: std.numeric.Int64 = -2;
            if negTwo.pow(3) != -8 { return 4 }

            // gcd
            let twelve: std.numeric.Int64 = 12;
            if twelve.gcd(8) != 4 { return 5 }
            let seventeen: std.numeric.Int64 = 17;
            if seventeen.gcd(13) != 1 { return 6 }
            let zero: std.numeric.Int64 = 0;
            if zero.gcd(5) != 5 { return 7 }
            let negTwelve: std.numeric.Int64 = -12;
            if negTwelve.gcd(8) != 4 { return 8 }

            // lcm
            let four: std.numeric.Int64 = 4;
            if four.lcm(6) != 12 { return 9 }
            if three.lcm(5) != 15 { return 10 }
            if zero.lcm(5) != 0 { return 11 }

            // clamp
            if five.clamp(0, 10) != 5 { return 12 }
            let negFive: std.numeric.Int64 = -5;
            if negFive.clamp(0, 10) != 0 { return 13 }
            let fifteen: std.numeric.Int64 = 15;
            if fifteen.clamp(0, 10) != 10 { return 14 }

            // successor
            if five.successor() != 6 { return 15 }
            let negOne: std.numeric.Int64 = -1;
            if negOne.successor() != 0 { return 16 }

            // predecessor
            if five.predecessor() != 4 { return 17 }
            if zero.predecessor() != -1 { return 18 }

            0
        }
