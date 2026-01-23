#include <stdio.h>
#include <termios.h>
#include <unistd.h>
#include <fcntl.h>
#include <stdint.h>

static struct termios orig_termios;
static int terminal_initialized = 0;

// Initialize terminal for non-blocking raw input
int32_t breakout_init_terminal(void) {
    if (terminal_initialized) return 0;

    struct termios raw;
    if (tcgetattr(STDIN_FILENO, &orig_termios) == -1) return -1;

    raw = orig_termios;
    raw.c_lflag &= ~(ICANON | ECHO);  // Disable canonical mode and echo
    raw.c_cc[VMIN] = 0;   // Don't wait for characters
    raw.c_cc[VTIME] = 0;  // No timeout

    if (tcsetattr(STDIN_FILENO, TCSANOW, &raw) == -1) return -1;

    // Set non-blocking
    int flags = fcntl(STDIN_FILENO, F_GETFL, 0);
    fcntl(STDIN_FILENO, F_SETFL, flags | O_NONBLOCK);

    terminal_initialized = 1;
    return 0;
}

// Restore terminal to original settings
int32_t breakout_restore_terminal(void) {
    if (!terminal_initialized) return 0;
    tcsetattr(STDIN_FILENO, TCSANOW, &orig_termios);
    terminal_initialized = 0;
    return 0;
}

// Check for keypress, returns:
//   -1 = no key pressed
//   Otherwise returns the key code
// For arrow keys: returns 1000 + direction (1=up, 2=down, 3=right, 4=left)
int32_t breakout_check_key(void) {
    unsigned char c;
    if (read(STDIN_FILENO, &c, 1) != 1) {
        return -1;  // No key
    }

    // Check for escape sequence (arrow keys)
    if (c == 27) {  // ESC
        unsigned char seq[2];
        if (read(STDIN_FILENO, &seq[0], 1) != 1) return 27;
        if (read(STDIN_FILENO, &seq[1], 1) != 1) return 27;

        if (seq[0] == '[') {
            switch (seq[1]) {
                case 'A': return 1001;  // Up arrow
                case 'B': return 1002;  // Down arrow
                case 'C': return 1003;  // Right arrow
                case 'D': return 1004;  // Left arrow
            }
        }
        return 27;  // Just ESC
    }

    return (int32_t)c;
}
