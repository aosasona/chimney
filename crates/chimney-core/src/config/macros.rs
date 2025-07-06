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
