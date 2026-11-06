// test: execution
// stdlib: true
// expect-exit: -1
// expect-stdout: step 1 done\nstep 2 done\n

module Test

// Standard output is block-buffered when it isn't a terminal and flushed
// by `atexit`. A fatal error traps, which skips `atexit`, so the lines
// printed before it were lost. Writing to standard error (as fatalError
// does) now flushes standard output first.
@main
func main() {
    println("step 1 done");
    println("step 2 done");
    let a: Array[Int64] = [1, 2, 3];
    let i = a.count + 5;
    println(a(i));
}
