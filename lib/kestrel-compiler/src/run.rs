//! Run result types.

/// Result of running a compiled program.
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Exit code of the program.
    pub exit_code: i32,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
}

impl RunResult {
    /// Create a new run result.
    pub fn new(exit_code: i32, stdout: String, stderr: String) -> Self {
        Self {
            exit_code,
            stdout,
            stderr,
        }
    }

    /// Check if the program exited successfully (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}
