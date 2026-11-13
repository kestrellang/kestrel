// Rosetta Code: The Twelve Days of Christmas
// https://rosettacode.org/wiki/The_Twelve_Days_of_Christmas
module Main

@main
func main() {
    let days = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth",
                "ninth", "tenth", "eleventh", "twelfth"];
    let gifts = ["A partridge in a pear tree.", "Two turtle doves", "Three french hens",
                 "Four calling birds", "Five golden rings", "Six geese a-laying",
                 "Seven swans a-swimming", "Eight maids a-milking", "Nine ladies dancing",
                 "Ten lords a-leaping", "Eleven pipers piping", "Twelve drummers drumming"];
    for (day, name) in days.iter().enumerate() {
        println("On the \(name) day of Christmas");
        println("My true love gave to me:");
        var g = day;
        while g >= 0 {
            if day > 0 and g == 0 {
                println("And a partridge in a pear tree.");
            } else {
                println(gifts(g));
            }
            g = g - 1;
        }
        println();
    }
}
