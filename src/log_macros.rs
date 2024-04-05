#[macro_export]
macro_rules! log_request {
    ($req:expr) => {
        use chrono::prelude::*;
        use hyper::Request;

        let req = $req as &Request<hyper::body::Incoming>;
        let utc_time = Utc::now().to_rfc3339();

        println!(
            "\x1b[95m[{}]\x1b[0m {} {} - {}",
            utc_time,
            req.method(),
            req.uri().path(),
            req.headers()
                .get("User-Agent")
                .unwrap_or(&"Unknown".parse().unwrap())
                .to_str()
                .unwrap_or("Unknown")
        );
    };
}

#[macro_export]
macro_rules! log_error {
    ($error:expr) => {
        eprintln!("\x1b[91m[Error] {}\x1b[0m", $error);
    };
}

#[macro_export]
macro_rules! log_warning {
    ($warning:expr) => {
        eprintln!("\x1b[93m[WARNING] {}\x1b[0m", $warning);
    };
}

#[macro_export]
macro_rules! log_info {
    ($info:expr) => {
        println!("\x1b[94m[INFO] {}\x1b[0m", $info);
    };
}
