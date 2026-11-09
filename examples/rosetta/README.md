# Rosetta Code in Kestrel

Solutions to 232 [Rosetta Code](https://rosettacode.org/wiki/Category:Programming_Tasks) tasks, one file per task. Each file starts with the task name and its wiki link, and has a matching `.out` file with the expected output.

The programs use only the standard library. Where a task normally reads a file (Anagrams, Letter frequency) a small embedded input keeps it self-contained, and tasks that use randomness seed `Lcg64` so their output is reproducible.

## Running them

```bash
cargo build --release --bin kestrel
examples/rosetta/run.sh                      # build, run and diff every task
examples/rosetta/run.sh fizzbuzz sudoku      # just these
OPT=2 examples/rosetta/run.sh                # with the LLVM backend
BLESS=1 examples/rosetta/run.sh new_task     # record new_task.out from its current output
```

`run.sh` builds each task into `$TMPDIR/kestrel-rosetta`, runs it with a 60 second limit, and prints `ok`, `WRONG`, `BUILD FAIL` or `NO .out` per task. Build logs and actual output are kept next to the binaries.

## Tasks

- [100 doors](100_doors.ks)
- [100 prisoners](one_hundred_prisoners.ks)
- [99 bottles of beer](99_bottles_of_beer.ks)
- [Abundant odd numbers](abundant_odd_numbers.ks)
- [Abundant, deficient and perfect number classifications](abundant_deficient_and_perfect_number_classifications.ks)
- [Accumulator factory](accumulator_factory.ks)
- [Ackermann function](ackermann_function.ks)
- [Almost prime](almost_prime.ks)
- [Amicable pairs](amicable_pairs.ks)
- [Anagrams](anagrams.ks)
- [Arithmetic evaluation](arithmetic_evaluation.ks)
- [Arithmetic-geometric mean](arithmetic_geometric_mean.ks)
- [Arithmetic/Complex](arithmetic_complex.ks)
- [Arithmetic/Integer](arithmetic_integer.ks)
- [Arithmetic/Rational](arithmetic_rational.ks)
- [Array concatenation](array_concatenation.ks)
- [Array length](array_length.ks)
- [Arrays](arrays.ks)
- [Associative array/Creation](associative_array_creation.ks)
- [Associative array/Iteration](associative_array_iteration.ks)
- [Attractive numbers](attractive_numbers.ks)
- [Averages (arithmetic mean, median, mode, standard deviation)](averages.ks)
- [Balanced brackets](balanced_brackets.ks)
- [Base64 encode data](base64_encode_data.ks)
- [Binary digits](binary_digits.ks)
- [Binary search](binary_search.ks)
- [Bitwise operations](bitwise_operations.ks)
- [Boolean values](boolean_values.ks)
- [Caesar cipher](caesar_cipher.ks)
- [Cartesian product of two or more lists](cartesian_product_of_two_or_more_lists.ks)
- [Catalan numbers](catalan_numbers.ks)
- [Character codes](character_codes.ks)
- [Chinese remainder theorem](chinese_remainder_theorem.ks)
- [Closures/Value capture](closures_value_capture.ks)
- [Combinations](combinations.ks)
- [Comma quibbling](comma_quibbling.ks)
- [Command-line arguments](command_line_arguments.ks)
- [Compound data type](compound_data_type.ks)
- [Conway's Game of Life](conways_game_of_life.ks)
- [Copy a string](copy_a_string.ks)
- [Count in octal](count_in_octal.ks)
- [Count occurrences of a substring](count_occurrences_of_a_substring.ks)
- [Count the coins](count_the_coins.ks)
- [CRC-32](crc_32.ks)
- [Currying](currying.ks)
- [Day of the week](day_of_the_week.ks)
- [Determinant and permanent](determinant_and_permanent.ks)
- [Determine if a string is numeric](determine_if_a_string_is_numeric.ks)
- [Digital root](digital_root.ks)
- [Dijkstra's algorithm](dijkstras_algorithm.ks)
- [Dot product](dot_product.ks)
- [Emirp primes](emirp_primes.ks)
- [Empty string](empty_string.ks)
- [Enumerations](enumerations.ks)
- [Equilibrium index](equilibrium_index.ks)
- [Ethiopian multiplication](ethiopian_multiplication.ks)
- [Evaluate binomial coefficients](evaluate_binomial_coefficients.ks)
- [Even or odd](even_or_odd.ks)
- [Exceptions](exceptions.ks)
- [Execute Brain****](execute_brain_fuck.ks)
- [Exponentiation operator](exponentiation_operator.ks)
- [Factorial](factorial.ks)
- [Factorial (100! with a hand-rolled bignum)](factorial_of_100.ks)
- [Factors of an integer](factors_of_an_integer.ks)
- [Fibonacci n-step number sequences](fibonacci_n_step_number_sequences.ks)
- [Fibonacci sequence](fibonacci_sequence.ks)
- [Five weekends](five_weekends.ks)
- [FizzBuzz](fizzbuzz.ks)
- [Flatten a list](flatten_a_list.ks)
- [Floyd's triangle](floyds_triangle.ks)
- [Floyd-Warshall algorithm](floyd_warshall_algorithm.ks)
- [Function composition](function_composition.ks)
- [Gaussian elimination](gaussian_elimination.ks)
- [Generate lower case ASCII alphabet](generate_lower_case_ascii_alphabet.ks)
- [Generic swap](generic_swap.ks)
- [Gray code](gray_code.ks)
- [Greatest common divisor](greatest_common_divisor.ks)
- [Greatest subsequential sum](greatest_subsequential_sum.ks)
- [Hailstone sequence](hailstone_sequence.ks)
- [Hamming numbers](hamming_numbers.ks)
- [Happy numbers](happy_numbers.ks)
- [Harshad or Niven series](harshad_or_niven_series.ks)
- [Hash from two arrays](hash_from_two_arrays.ks)
- [Hello world/Newline omission](hello_world_newline_omission.ks)
- [Hello world/Text](hello_world.ks)
- [Higher-order functions](higher_order_functions.ks)
- [Hofstadter Q sequence](hofstadter_q_sequence.ks)
- [Horner's rule for polynomial evaluation](horners_rule_for_polynomial_evaluation.ks)
- [Huffman coding](huffman_coding.ks)
- [Identity matrix](identity_matrix.ks)
- [Increment a numerical string](increment_a_numerical_string.ks)
- [Integer comparison](integer_comparison.ks)
- [Integer overflow](integer_overflow.ks)
- [ISBN13 check digit](isbn13_check_digit.ks)
- [Isqrt (integer square root) of X](integer_sqrt.ks)
- [Jewels and stones](jewels_and_stones.ks)
- [Josephus problem](josephus_problem.ks)
- [Kaprekar numbers](kaprekar_numbers.ks)
- [Knapsack problem/0-1](knapsack_problem_0_1.ks)
- [Knight's tour](knights_tour.ks)
- [Kolakoski sequence](kolakoski_sequence.ks)
- [Langton's ant](langtons_ant.ks)
- [Last Friday of each month](last_friday_of_each_month.ks)
- [Leap year](leap_year.ks)
- [Least common multiple](least_common_multiple.ks)
- [Leonardo numbers](leonardo_numbers.ks)
- [Letter frequency](letter_frequency.ks)
- [Levenshtein distance](levenshtein_distance.ks)
- [Long multiplication](long_multiplication.ks)
- [Longest common subsequence](longest_common_subsequence.ks)
- [Longest increasing subsequence](longest_increasing_subsequence.ks)
- [Look-and-say sequence](look_and_say_sequence.ks)
- [Loops/Continue](loops_continue.ks)
- [Loops/Do-while](loops_do_while.ks)
- [Loops/Downward for](loops_downward_for.ks)
- [Loops/For with a specified step](loops_for_with_a_specified_step.ks)
- [Loops/For](loops_for.ks)
- [Loops/N plus one half](loops_n_plus_one_half.ks)
- [Loops/Nested](loops_nested.ks)
- [Lucas-Lehmer test](lucas_lehmer_test.ks)
- [Ludic numbers](ludic_numbers.ks)
- [Luhn test of credit card numbers](luhn_test_of_credit_card_numbers.ks)
- [LZW compression](lzw_compression.ks)
- [Magic squares of odd order](magic_squares_of_odd_order.ks)
- [Mandelbrot set](mandelbrot_set.ks)
- [Matrix multiplication](matrix_multiplication.ks)
- [Matrix transposition](matrix_transposition.ks)
- [Maze generation](maze_generation.ks)
- [Middle three digits](middle_three_digits.ks)
- [Modular exponentiation (with 64-bit operands)](modular_exponentiation.ks)
- [Modular inverse](modular_inverse.ks)
- [Monte Carlo methods](monte_carlo_methods.ks)
- [Monty Hall problem](monty_hall_problem.ks)
- [Multiplication tables](multiplication_tables.ks)
- [Mutual recursion (Hofstadter Female and Male sequences)](mutual_recursion.ks)
- [Möbius function](mobius_function.ks)
- [N'th](nth.ks)
- [N-queens problem](n_queens_problem.ks)
- [Named parameters](named_parameters.ks)
- [Narcissistic decimal number](narcissistic_decimal_number.ks)
- [Nth root](nth_root.ks)
- [Null object](null_object.ks)
- [Number names](number_names.ks)
- [Numerical integration](numerical_integration.ks)
- [Optional parameters](optional_parameters.ks)
- [Palindrome detection](palindrome_detection.ks)
- [Pangram checker](pangram_checker.ks)
- [Parsing/RPN calculator algorithm](rpn_calculator_algorithm.ks)
- [Parsing/Shunting-yard algorithm](shunting_yard_algorithm.ks)
- [Pascal's triangle](pascals_triangle.ks)
- [Pattern matching (red-black tree insertion)](pattern_matching.ks)
- [Perfect numbers](perfect_numbers.ks)
- [Permutations](permutations.ks)
- [Pernicious numbers](pernicious_numbers.ks)
- [Pi](pi.ks)
- [Polymorphism](polymorphism.ks)
- [Power set](power_set.ks)
- [Primality by trial division](primality_by_trial_division.ks)
- [Prime decomposition](prime_decomposition.ks)
- [Priority queue](priority_queue.ks)
- [Proper divisors](proper_divisors.ks)
- [Quaternion type](quaternion_type.ks)
- [Queue/Definition](queue_definition.ks)
- [Real constants and functions](real_constants_and_functions.ks)
- [Recaman's sequence](recamans_sequence.ks)
- [Remove duplicate elements](remove_duplicate_elements.ks)
- [Repeat a string](repeat_a_string.ks)
- [Reverse a string](reverse_a_string.ks)
- [Reverse words in a string](reverse_words_in_a_string.ks)
- [Roman numerals/Decode](roman_numerals_decode.ks)
- [Roman numerals/Encode](roman_numerals_encode.ks)
- [Roots of a quadratic function](roots_of_a_quadratic_function.ks)
- [Rot-13](rot13.ks)
- [Run-length encoding](run_length_encoding.ks)
- [Self-describing numbers](self_describing_numbers.ks)
- [Semiprime](semiprime.ks)
- [Set](set.ks)
- [SHA-256](sha_256.ks)
- [Show ASCII table](show_ascii_table.ks)
- [Sierpinski carpet](sierpinski_carpet.ks)
- [Sierpinski triangle](sierpinski_triangle.ks)
- [Sieve of Eratosthenes](sieve_of_eratosthenes.ks)
- [Singly-linked list/Traversal](singly_linked_list_traversal.ks)
- [Sort an array of composite structures](sort_an_array_of_composite_structures.ks)
- [Sort stability](sort_stability.ks)
- [Sort using a custom comparator](sort_using_a_custom_comparator.ks)
- [Sorting algorithms/Bubble sort](bubble_sort.ks)
- [Sorting algorithms/Cocktail sort](cocktail_sort.ks)
- [Sorting algorithms/Comb sort](comb_sort.ks)
- [Sorting algorithms/Counting sort](counting_sort.ks)
- [Sorting algorithms/Gnome sort](gnome_sort.ks)
- [Sorting algorithms/Heapsort](heapsort.ks)
- [Sorting algorithms/Insertion sort](insertion_sort.ks)
- [Sorting algorithms/Merge sort](merge_sort.ks)
- [Sorting algorithms/Pancake sort](pancake_sort.ks)
- [Sorting algorithms/Quicksort](quicksort.ks)
- [Sorting algorithms/Radix sort](radix_sort.ks)
- [Sorting algorithms/Selection sort](selection_sort.ks)
- [Sorting algorithms/Shell sort](shell_sort.ks)
- [Spiral matrix](spiral_matrix.ks)
- [Square-free integers](square_free_integers.ks)
- [Stack](stack.ks)
- [Stern-Brocot sequence](stern_brocot_sequence.ks)
- [String case](string_case.ks)
- [String concatenation](string_concatenation.ks)
- [String length](string_length.ks)
- [String matching](string_matching.ks)
- [Strip a set of characters from a string](strip_a_set_of_characters_from_a_string.ks)
- [Strip whitespace from a string/Top and tail](strip_whitespace_from_a_string.ks)
- [Substring](substring.ks)
- [Sudoku](sudoku.ks)
- [Sum digits of an integer](sum_digits_of_an_integer.ks)
- [Sum multiples of 3 and 5](sum_multiples_of_3_and_5.ks)
- [Sum of a series](sum_of_a_series.ks)
- [Sum of squares](sum_of_squares.ks)
- [Temperature conversion](temperature_conversion.ks)
- [The Twelve Days of Christmas](the_twelve_days_of_christmas.ks)
- [Thue-Morse](thue_morse.ks)
- [Tokenize a string](tokenize_a_string.ks)
- [Topological sort](topological_sort.ks)
- [Totient function](totient_function.ks)
- [Towers of Hanoi](towers_of_hanoi.ks)
- [Tree traversal](tree_traversal.ks)
- [Trigonometric functions](trigonometric_functions.ks)
- [Twin primes](twin_primes.ks)
- [Two sum](two_sum.ks)
- [Van der Corput sequence](van_der_corput_sequence.ks)
- [Van Eck sequence](van_eck_sequence.ks)
- [Vector products](vector_products.ks)
- [Vigenère cipher](vigenere_cipher.ks)
- [Zeckendorf number representation](zeckendorf_number_representation.ks)
- [Zig-zag matrix](zig_zag_matrix.ks)

## What writing these turned up

Every task here compiles and matches its `.out` at `-O0` and `-O2`.

### Compiler bugs (all fixed)

Writing the tasks turned up nine compiler bugs. Each one has a regression test in `lib/kestrel-test-suite/testdata`. All of them are fixed on nightly, and the examples no longer work around them.

| Bug | Fixed in | Test |
|---|---|---|
| A second trailing closure in a chain (`xs.iter().filter { … }.map { … }`) wasn't parsed. | #263 | `expressions/closures/trailing_closure_second_in_chain.ks` |
| `.filter { … }` on a continuation line of a multi-line chain wasn't parsed. | #263 | `expressions/closures/trailing_closure_on_continuation_line.ks` |
| `if flags(i)` on an `Array[Bool]` was E101 "expected Bool, found &Bool". | #263 | `expressions/control_flow/if_condition_bool_array_element.ks` |
| `match`/`if` branches mixing `"literal"` and `"\(interpolated)"` were E109 without a `: String` annotation. | #263 | `inference/branch_string_literal_and_interpolation.ks` |
| `String(from:)` on chars resolved to `init(from: CString)`. | #261, #266 | `stdlib/string/string_from_char_iterable.ks` |
| An unannotated `var i = 2` used with its own operator result (`var j = i + i; j + i`) stayed an "integer literal". | #263 | `inference/var_integer_literal_used_as_index.ks` |
| A `Float64` function ending in `loop { … return x … }` failed Cranelift/LLVM verification. | #263 | `codegen/loops/tail_loop_returning_float.ks` |
| A `=> { … }` match arm without a comma, followed by a `.Case =>` arm, parsed as `{ … }.Case`. | #263 | `expressions/match/block_arm_then_implicit_member_arm.ks` |
| A closure inside an interpolation hole defaulted its literals too early (E109 "expected Int64 got UInt32"). | #263 | `expressions/strings/closure_in_interpolation_hole_infers.ks` |

### Language and standard library gaps

- **No tuple destructuring assignment.** `(a, b) = (b, a + b)` is E202, so Fibonacci and Euclid need a temporary. `var (a, b) = …` does work.
- **Tuples aren't `Formattable`**, so `println("\(pair)")` doesn't compile.
- **No descending ranges or `stride`.** `(1..=99).reversed()` doesn't exist; count up and subtract, or use `while`.
- **`Char` can't be a range bound for iteration.** `('a'..='z').iter()` is "Char !: ClosedRangeConstructible", though char ranges work as `match` patterns.
- **No integer `min`/`max`.** Only `clamp` exists on `Int64`; Hamming numbers compares by hand.
- **Iterators aren't `Iterable`.** `Dictionary(from: a.iter().zip(b.iter()))`, `Set(from: …)` and `String(from: …)` need a `.collect()` first.
- **Closure parameters can't destructure tuples.** `zip(…).map { (x, y) in x * y }` is E109; write `it.0 * it.1`.
- **Lazy adapters can't capture a closure parameter.** `Iterator.map`/`filter` take `escaping` closures, so inside `func f(g: (Float64) -> Float64)` the body `(0..<n).iter().map { g(it) }` is E624. Numerical integration uses plain loops.
- **`StringSlice` can't be printed or interpolated** ("StringSlice !: Static"). `trimmed()` and `split()` pieces need `.toOwned()` first.
- **String `split` views have no `toArray()`** (Array's do): `s.split(",").iter().map { it.toOwned() }.collect()`.
- **Overloads can't differ only by parameter type.** `power(Int64, Int64)` and `power(Float64, Int64)` are E426.
- **Global `let` needs a type annotation** (`let names: Array[String] = […]`).
- **`1e-12` doesn't lex**; `1.0e-12` does.
- **Label inconsistencies.** `Array.map(f)` vs `Iterator.map(as:)`; `Int64.random(in:using:)` vs `Float64.random(from:to:)`.
- **`GraphemesView` has no `reversed`** (`CharsView` does).
- **`dict(key) = value` is E214** ("cannot assign to failable place"); use `insert`.

### Confusing diagnostics

- Calling `init(n: Int64)` as `T(n: 8)` is E113 "no matching overload" with no hint that single-name parameters are unlabeled.
- `!flag` on a `Bool` says "no member 'bitwiseNot'"; it could suggest `not`.
- `Int64.random(from:to:using:)` (no such overload) is E116 "'random' is a static member and cannot be used on an instance".
- `func compare(other o: T)` gives E454 "does not implement method 'compare'" without mentioning the label.
- `0.5 |> _.asin()` suggests `_.asin(_)`, which doesn't help.
- `docs/language/syntax.md` shows `func render[T: Drawable](…)`, but inline bounds are a parse error (E800); only `where T: Drawable` works.
