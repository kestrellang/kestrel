// Rosetta Code: Cartesian product of two or more lists
// https://rosettacode.org/wiki/Cartesian_product_of_two_or_more_lists
module Main

func product(lists: Array[Array[Int64]]) -> Array[Array[Int64]] {
    var result: Array[Array[Int64]] = [[]];
    for list in lists {
        var next = Array[Array[Int64]]();
        for prefix in result {
            for x in list {
                var row = prefix;
                row.append(x);
                next.append(row);
            }
        }
        result = next;
    }
    result
}

@main
func main() {
    println(product([[1, 2], [3, 4]]));
    println(product([[3, 4], [1, 2]]));
    println(product([[1, 2], []]));
    println(product([[], [1, 2]]));
    println(product([[1776, 1789], [7, 12], [4, 14, 23], [0, 1]]));
    println(product([[1, 2, 3], [30], [500, 100]]));
    println(product([[1, 2, 3], [], [500, 100]]));
}
