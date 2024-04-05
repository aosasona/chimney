use path_absolutize::*;
use std::path::PathBuf;

use chimney::config::*;
use toml::toml;

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

#[test]
fn test_root_path() {
    let raw_config: String = toml! {
        host = "0.0.0.0"
        post = 80
        domain_names = []
        enable_logging = true
        root = "public"
        fallback_document = "index.html"
        mode = "single"
    }
    .to_string();

    let config = parse_config(&PathBuf::new(), raw_config).expect("Failed to parse config");
    let public_pathbuf: PathBuf = Root::Path("public".to_string()).into();
    let public_path = public_pathbuf
        .absolutize()
        .expect("Could not absolutize")
        .to_string_lossy()
        .to_string();

    assert_eq!(config.root.get_path(), public_path);
    assert_eq!(config.root.get_ignore_matches(), None);
}

#[test]
fn test_root_config() {
    let raw_config: String = toml! {
        host = "0.0.0.0"
        post = 80
        domain_names = []
        enable_logging = true
        root = {
            path = "public"
        }
        fallback_document = "index.html"
        mode = "single"
    }
    .to_string();

    let config = parse_config(&PathBuf::new(), raw_config).expect("Failed to parse config");
    let public_pathbuf: PathBuf = Root::Path("public".to_string()).into();
    let public_path = public_pathbuf
        .absolutize()
        .expect("Could not absolutize")
        .to_string_lossy()
        .to_string();

    assert_eq!(config.root.get_path(), public_path);
    assert_eq!(config.root.get_ignore_matches(), Some(vec![]));
}

#[test]
fn test_root_config_multi() {
    let raw_config: String = toml! {
        host = "0.0.0.0"
        post = 80
        domain_names = []
        enable_logging = true
        root = {
            path = "public",
            ignore_matches = ["foo", "bar"]
        }
        fallback_document = "index.html"
        mode = "multi"
    }
    .to_string();

    let config = parse_config(&PathBuf::new(), raw_config).expect("Failed to parse config");
    let public_pathbuf: PathBuf = Root::Path("public".to_string()).into();
    let public_path = public_pathbuf
        .absolutize()
        .expect("Could not absolutize")
        .to_string_lossy()
        .to_string();

    assert_eq!(config.root.get_path(), public_path);
    assert_eq!(
        config.root.get_ignore_matches(),
        Some(vec!["foo".to_string(), "bar".to_string()])
    );
}
