// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- replaced() (non-mutating) ----
            let s1: std.text.String = "hello world hello";
            let r1 = s1.replaced("hello", with: "hi");
            if r1.isEqual(to: "hi world hi") == false { return 1 }
            // Original unchanged
            if s1.isEqual(to: "hello world hello") == false { return 2 }

            // Replace with longer string
            let s2: std.text.String = "aaa";
            let r2 = s2.replaced("a", with: "bb");
            if r2.isEqual(to: "bbbbbb") == false { return 3 }

            // Replace with shorter string
            let s3: std.text.String = "hello";
            let r3 = s3.replaced("ll", with: "l");
            if r3.isEqual(to: "helo") == false { return 4 }

            // Replace no match
            let s4: std.text.String = "hello";
            let r4 = s4.replaced("xyz", with: "abc");
            if r4.isEqual(to: "hello") == false { return 5 }

            // ---- replace() (mutating) ----
            var s5: std.text.String = "foo bar foo";
            s5.replace("foo", with: "baz");
            if s5.isEqual(to: "baz bar baz") == false { return 6 }

            // ---- split(separator:) ----
            let csv: std.text.String = "a,b,c";
            let parts = csv.split(",").collect();
            if parts.count != 3 { return 7 }
            if parts(unchecked: 0).toOwned().isEqual(to: "a") == false { return 8 }
            if parts(unchecked: 1).toOwned().isEqual(to: "b") == false { return 9 }
            if parts(unchecked: 2).toOwned().isEqual(to: "c") == false { return 10 }

            // Split with no separator found
            let noSep: std.text.String = "hello";
            let noParts = noSep.split(",").collect();
            if noParts.count != 1 { return 11 }
            if noParts(unchecked: 0).toOwned().isEqual(to: "hello") == false { return 12 }

            // Split with adjacent separators
            let adj: std.text.String = "a,,b";
            let adjParts = adj.split(",").collect();
            if adjParts.count != 3 { return 13 }
            if adjParts(unchecked: 1).toOwned().isEqual(to: "") == false { return 14 }

            // ---- split(matching:) ----
            let s6: std.text.String = "hello world\tthere";
            let wsParts = s6.split(matching: { (c) in c.isWhitespace() }).collect();
            if wsParts.count != 3 { return 15 }
            if wsParts(unchecked: 0).toOwned().isEqual(to: "hello") == false { return 16 }
            if wsParts(unchecked: 1).toOwned().isEqual(to: "world") == false { return 17 }
            if wsParts(unchecked: 2).toOwned().isEqual(to: "there") == false { return 18 }

            0
        }
