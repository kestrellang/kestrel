// Print support functions for Kestrel phase12 examples
// Provides printf-based output for the Print module

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>

// Print a string given pointer and length
int kestrel_print_string(const int8_t* ptr, int64_t len) {
    int result = (int)fwrite(ptr, 1, (size_t)len, stdout);
    fflush(stdout);
    return result;
}

// Print an integer (Int = Int64)
int kestrel_print_int(int64_t value) {
    int result = printf("%lld", (long long)value);
    fflush(stdout);
    return result;
}

// Print a float (Float = Float64 = double)
int kestrel_print_float(double value) {
    int result = printf("%g", value);
    fflush(stdout);
    return result;
}

// Print a boolean
int kestrel_print_bool(bool value) {
    int result = printf("%s", value ? "true" : "false");
    fflush(stdout);
    return result;
}

// Print a newline (dummy parameter for Kestrel compatibility)
int kestrel_print_newline(int64_t dummy) {
    (void)dummy;  // Unused
    int result = printf("\n");
    fflush(stdout);
    return result;
}
