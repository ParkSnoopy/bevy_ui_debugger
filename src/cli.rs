use std::env;

/// Command-line flag that opts the debugger in.
pub const FLAG: &str = "--ui-debug";

/// Environment variable that opts the debugger in when set to anything but empty or `0`.
pub const ENV: &str = "BEVY_UI_DEBUG";

/// Returns true when the process was started with `--ui-debug` or with `BEVY_UI_DEBUG` set.
///
/// A host that parses its command line strictly (for example clap `derive`) should declare
/// the flag itself and call this only as a fallback.
pub fn is_requested() -> bool {
    if env::args_os().skip(1).any(|arg| arg == FLAG) {
        return true;
    }
    env::var_os(ENV).is_some_and(|value| !value.is_empty() && value != "0")
}
