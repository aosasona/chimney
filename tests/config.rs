use std::path::PathBuf;

use chimney::config::*;

macro_rules! config_str {
    ($value:expr) => {
        format!(
            r#"
    host = "0.0.0.0"
    port = 80
    domain_names = []
    enable_logging = true
    root = "public"
    fallback_document = "index.html"
    mode = "{}"
    "#,
            $value
        )
    };
}

#[test]
fn test_invalid_mode() {
    assert!(parse_config(&PathBuf::new(), config_str!("multi")).is_ok() == true);
    assert!(parse_config(&PathBuf::new(), config_str!("MULTI")).is_ok() == false);

    assert!(parse_config(&PathBuf::new(), config_str!("single")).is_ok() == true);
    assert!(parse_config(&PathBuf::new(), config_str!("SINGLE")).is_ok() == false);

    assert!(parse_config(&PathBuf::new(), config_str!("invalid")).is_ok() == false);
}
