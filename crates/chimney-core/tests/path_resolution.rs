// Path resolution tests
// These tests verify that site.root is properly configured and handled
// The actual path resolution logic is integration-tested with real filesystems

use chimney::config::{Config, Site};

fn create_test_site_toml(root: &str, domain: &str) -> String {
    format!(
        r#"
root = "{}"
domain_names = ["{}"]
"#,
        root, domain
    )
}

#[test]
fn test_site_root_config_parsing() {
    // Test that site root is correctly parsed from TOML
    let site_toml = create_test_site_toml("public/dist", "example.com");
    let site = Site::from_string("example".to_string(), &site_toml).unwrap();

    assert_eq!(site.root, "public/dist");
    assert_eq!(site.domain_names, vec!["example.com"]);
    assert_eq!(site.name, "example");
}

#[test]
fn test_site_root_default() {
    // Test that site root defaults to "." when not specified
    let site_toml = r#"
domain_names = ["example.com"]
"#;
    let site = Site::from_string("example".to_string(), site_toml).unwrap();

    assert_eq!(site.root, ".");
}

#[test]
fn test_site_root_relative_path() {
    // Test relative path configuration
    let site_toml = create_test_site_toml("public", "example.com");
    let site = Site::from_string("example".to_string(), &site_toml).unwrap();

    assert_eq!(site.root, "public");
}

#[test]
fn test_site_root_nested_path() {
    // Test nested relative path configuration
    let site_toml = create_test_site_toml("dist/public", "example.com");
    let site = Site::from_string("example".to_string(), &site_toml).unwrap();

    assert_eq!(site.root, "dist/public");
}

#[test]
fn test_site_root_absolute_path() {
    // Test absolute path configuration (for backwards compatibility)
    let site_toml = create_test_site_toml("/absolute/path/to/site", "example.com");
    let site = Site::from_string("example".to_string(), &site_toml).unwrap();

    assert_eq!(site.root, "/absolute/path/to/site");
}

#[test]
fn test_multiple_sites_with_different_roots() {
    // Test multiple sites can have different root configurations
    let mut config = Config::default();
    config.sites_directory = "/var/www/sites".to_string();

    let site1_toml = create_test_site_toml("public", "site1.com");
    let site1 = Site::from_string("site1".to_string(), &site1_toml).unwrap();

    let site2_toml = create_test_site_toml("dist", "site2.com");
    let site2 = Site::from_string("site2".to_string(), &site2_toml).unwrap();

    let site3_toml = create_test_site_toml(".", "site3.com");
    let site3 = Site::from_string("site3".to_string(), &site3_toml).unwrap();

    config.sites.add(site1).unwrap();
    config.sites.add(site2).unwrap();
    config.sites.add(site3).unwrap();

    let loaded_site1 = config.sites.find_by_hostname("site1.com").unwrap();
    let loaded_site2 = config.sites.find_by_hostname("site2.com").unwrap();
    let loaded_site3 = config.sites.find_by_hostname("site3.com").unwrap();

    assert_eq!(loaded_site1.root, "public");
    assert_eq!(loaded_site2.root, "dist");
    assert_eq!(loaded_site3.root, ".");
}

#[test]
fn test_sites_directory_config() {
    // Test sites_directory configuration
    let mut config = Config::default();
    // Default is current_dir + "/sites"
    assert!(config.sites_directory.ends_with("/sites"));

    config.sites_directory = "/var/www/sites".to_string();
    assert_eq!(config.sites_directory, "/var/www/sites");
}

#[test]
fn test_site_root_preserves_value() {
    // Verify that site.root value is preserved exactly as configured
    let test_cases = vec![
        (".", "."),
        ("public", "public"),
        ("dist/build", "dist/build"),
        ("./relative/path", "./relative/path"),
        ("/absolute/path", "/absolute/path"),
    ];

    for (input, expected) in test_cases {
        let site_toml = create_test_site_toml(input, "example.com");
        let site = Site::from_string("test".to_string(), &site_toml).unwrap();
        assert_eq!(
            site.root, expected,
            "Site root should preserve value: {} -> {}",
            input, expected
        );
    }
}

#[test]
fn test_site_can_be_added_to_config() {
    // Verify sites with different roots can be added to config
    let mut config = Config::default();

    for i in 1..=5 {
        let root = format!("root{}", i);
        let domain = format!("site{}.com", i);
        let site_toml = create_test_site_toml(&root, &domain);
        let site = Site::from_string(format!("site{}", i), &site_toml).unwrap();
        config.sites.add(site).unwrap();
    }

    // Verify all sites were added
    for i in 1..=5 {
        let domain = format!("site{}.com", i);
        let site = config.sites.find_by_hostname(&domain).unwrap();
        assert_eq!(site.root, format!("root{}", i));
    }
}
