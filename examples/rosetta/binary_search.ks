// Rosetta Code: Binary search
// https://rosettacode.org/wiki/Binary_search
module Main

func binarySearch[T](haystack: Array[T], needle: T) -> Int64? where T: Comparable {
    var lo: Int64 = 0;
    var hi = haystack.count - 1;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let value = haystack(mid);
        if value < needle {
            lo = mid + 1;
        } else if value > needle {
            hi = mid - 1;
        } else {
            return .Some(mid);
        }
    }
    .None
}

@main
func main() {
    let a = [1, 3, 4, 7, 9, 11, 15, 22, 30];
    for needle in [7, 1, 30, 8] {
        match binarySearch(a, needle) {
            .Some(i) => println("\(needle) found at index \(i)"),
            .None => println("\(needle) not found"),
        }
    }
    // Arrays also have a built-in binary search.
    println("built-in: \(a.binarySearch(22) ?? -1)");
}
