// Debug tracing: set VERBOSE_DEBUG_OUTPUT=1 to enable
macro_rules! debug_trace {
    ($($arg:tt)*) => {
        if $crate::verbose_debug_enabled() {
            eprintln!($($arg)*);
        }
    };
}

pub(crate) fn verbose_debug_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("VERBOSE_DEBUG_OUTPUT").is_ok())
}

pub mod attributes;
pub mod behavior;
pub mod builtins;
pub mod error;
pub mod expr;
pub mod language;
pub mod operators;
pub mod pattern;
pub mod stmt;
pub mod symbol;
pub mod ty;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
