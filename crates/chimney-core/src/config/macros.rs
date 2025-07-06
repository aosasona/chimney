#[macro_export]
macro_rules! with_leading_slash {
    ($path:expr) => {
        if $path.starts_with('/') {
            $path.to_string()
        } else {
            format!("/{}", $path)
        }
    };
}

#[macro_export]
macro_rules! config_log_debug {
    ($target:expr, $($arg:tt)*) => {
        if cfg!(debug_assertions) {
            use chrono::Utc;
            const GREEN: &str = "\x1b[34m";
            const DIM: &str = "\x1b[2m";
            const RESET: &str = "\x1b[0m";
            let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

            println!(
                "{dim}[{reset}{timestamp} {green}DEBUG{reset} {target}{dim}]{reset} {}",
                format!($($arg)*),
                dim = DIM,
                green = GREEN,
                reset = RESET,
                timestamp = timestamp,
                target = $target
            );
        }
    };
}

#[macro_export]
macro_rules! config_log_warn {
    ($target:expr, $($arg:tt)*) => {
        if cfg!(debug_assertions) {
            use chrono::Utc;
            let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

            const DIM: &str = "\x1b[2m";
            const YELLOW: &str = "\x1b[1;33m";
            const RESET: &str = "\x1b[0m";

            eprintln!(
                "{dim}[{reset}{timestamp} {yellow}WARN{reset} {target}{dim}]{reset} {}",
                format!($($arg)*),
                dim = DIM,
                yellow = YELLOW,
                reset = RESET,
                timestamp = timestamp,
                target = $target
            );
        }
    };
}
