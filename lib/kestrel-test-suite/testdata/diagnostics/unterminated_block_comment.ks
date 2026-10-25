// test: diagnostics

module Test

func kept() -> Int64 { 1 }

/* this comment never closes, so everything after it is part of it // ERROR: unterminated block comment
func lost() -> Int64 { 2 }
