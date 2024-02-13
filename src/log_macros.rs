#[macro_export]
macro_rules! log_request {
    ($req:expr) => {
        use hyper::Request;
        use std::time::SystemTime;

        let req = $req as &Request<hyper::body::Incoming>;

        if let Ok(unix_time) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            println!(
                "{}",
                format!(
                    r#"{{"time": {}, "method": "{}", "path": "{}"}}"#,
                    unix_time.as_secs(),
                    req.method(),
                    req.uri().path(),
                )
            );
        }
    };
}
