// skip: included file, not a standalone test
module Test

private func privateHelper() -> Int64 { 1 }
fileprivate func fileprivateHelper() -> Int64 { 2 }
fileprivate struct FileprivateBox { var v: Int64 }
