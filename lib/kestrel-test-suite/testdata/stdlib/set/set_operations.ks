// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Setup two sets: a = {1, 2, 3}, b = {2, 3, 4}
            var a = std.collections.Set[std.numeric.Int64]();
            let _ = a.insert(1);
            let _ = a.insert(2);
            let _ = a.insert(3);

            var b = std.collections.Set[std.numeric.Int64]();
            let _ = b.insert(2);
            let _ = b.insert(3);
            let _ = b.insert(4);

            // Test union() - non-mutating
            let u = a.union(b);
            if u.count != 4 { return 1 }
            if u.contains(1) == false { return 2 }
            if u.contains(4) == false { return 3 }

            // Test intersection() - non-mutating
            let inter = a.intersection(b);
            if inter.count != 2 { return 4 }
            if inter.contains(2) == false { return 5 }
            if inter.contains(3) == false { return 6 }
            if inter.contains(1) { return 7 }

            // Test difference() - non-mutating
            let diff = a.difference(b);
            if diff.count != 1 { return 8 }
            if diff.contains(1) == false { return 9 }
            if diff.contains(2) { return 10 }

            // Test symmetricDifference() - non-mutating
            let symDiff = a.symmetricDifference(b);
            if symDiff.count != 2 { return 11 }
            if symDiff.contains(1) == false { return 12 }
            if symDiff.contains(4) == false { return 13 }
            if symDiff.contains(2) { return 14 }

            // Test formUnion() - mutating
            var fu = std.collections.Set[std.numeric.Int64]();
            let _ = fu.insert(1);
            let _ = fu.insert(2);
            fu.formUnion(b);
            if fu.count != 4 { return 15 }
            if fu.contains(4) == false { return 16 }

            // Test formIntersection() - mutating
            var fi = std.collections.Set[std.numeric.Int64]();
            let _ = fi.insert(1);
            let _ = fi.insert(2);
            let _ = fi.insert(3);
            fi.formIntersection(b);
            if fi.count != 2 { return 17 }
            if fi.contains(2) == false { return 18 }
            if fi.contains(1) { return 19 }

            // Test formDifference() - mutating
            var fd = std.collections.Set[std.numeric.Int64]();
            let _ = fd.insert(1);
            let _ = fd.insert(2);
            let _ = fd.insert(3);
            fd.formDifference(b);
            if fd.count != 1 { return 20 }
            if fd.contains(1) == false { return 21 }

            // Test formSymmetricDifference() - mutating
            var fsd = std.collections.Set[std.numeric.Int64]();
            let _ = fsd.insert(1);
            let _ = fsd.insert(2);
            let _ = fsd.insert(3);
            fsd.formSymmetricDifference(b);
            if fsd.count != 2 { return 22 }
            if fsd.contains(1) == false { return 23 }
            if fsd.contains(4) == false { return 24 }

            // Original sets unchanged
            if a.count != 3 { return 25 }
            if b.count != 3 { return 26 }

            0
        }
