use chimney::config::{Https, RedirectRule, SiteBuilder};

#[test]
fn test_site_builder_basic() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .build();

    assert_eq!(site.name, "my-site");
    assert_eq!(site.domain_names, vec!["example.com"]);
    assert_eq!(site.root, ".");
}

#[test]
fn test_site_builder_multiple_domains() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .domain("www.example.com")
        .domain("api.example.com")
        .build();

    assert_eq!(site.domain_names.len(), 3);
    assert!(site.domain_names.contains(&"example.com".to_string()));
    assert!(site.domain_names.contains(&"www.example.com".to_string()));
    assert!(site.domain_names.contains(&"api.example.com".to_string()));
}

#[test]
fn test_site_builder_domains_batch() {
    let site = SiteBuilder::new("my-site")
        .domains(["example.com", "www.example.com"])
        .build();

    assert_eq!(site.domain_names.len(), 2);
}

#[test]
fn test_site_builder_no_duplicate_domains() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .domain("example.com")
        .domains(["example.com", "other.com"])
        .build();

    assert_eq!(site.domain_names.len(), 2);
    assert!(site.domain_names.contains(&"example.com".to_string()));
    assert!(site.domain_names.contains(&"other.com".to_string()));
}

#[test]
fn test_site_builder_root() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .root("./public")
        .build();

    assert_eq!(site.root, "./public");
}

#[test]
fn test_site_builder_fallback_file() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .fallback_file("index.html")
        .build();

    assert_eq!(site.fallback_file, Some("index.html".to_string()));
}

#[test]
fn test_site_builder_default_index_file() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .default_index_file("index.htm")
        .build();

    assert_eq!(site.default_index_file, Some("index.htm".to_string()));
}

#[test]
fn test_site_builder_response_headers() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .response_header("X-Frame-Options", "DENY")
        .response_header("X-Content-Type-Options", "nosniff")
        .build();

    assert_eq!(site.response_headers.len(), 2);
    assert_eq!(site.response_headers.get("X-Frame-Options"), Some(&"DENY".to_string()));
    assert_eq!(site.response_headers.get("X-Content-Type-Options"), Some(&"nosniff".to_string()));
}

#[test]
fn test_site_builder_response_headers_batch() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .response_headers([
            ("X-Frame-Options", "DENY"),
            ("X-Content-Type-Options", "nosniff"),
        ])
        .build();

    assert_eq!(site.response_headers.len(), 2);
}

#[test]
fn test_site_builder_redirect() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .redirect("/old", "/new")
        .build();

    assert_eq!(site.redirects.len(), 1);
    let rule = site.redirects.get("/old").unwrap();
    assert_eq!(rule.target(), "/new");
}

#[test]
fn test_site_builder_redirect_rule() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .redirect_rule("/old", RedirectRule::new("/new".to_string(), true, false))
        .build();

    assert_eq!(site.redirects.len(), 1);
    let rule = site.redirects.get("/old").unwrap();
    assert_eq!(rule.target(), "/new");
    assert!(rule.is_temporary());
}

#[test]
fn test_site_builder_rewrite() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .rewrite("/api/*", "/backend/api/$1")
        .build();

    assert_eq!(site.rewrites.len(), 1);
    let rule = site.rewrites.get("/api/*").unwrap();
    assert_eq!(rule.target(), "/backend/api/$1");
}

#[test]
fn test_site_builder_manual_cert() {
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .manual_cert("./certs/cert.pem", "./certs/key.pem")
        .build();

    assert!(site.https_config.is_some());
    let https = site.https_config.unwrap();
    assert_eq!(https.cert_file, Some("./certs/cert.pem".to_string()));
    assert_eq!(https.key_file, Some("./certs/key.pem".to_string()));
    assert!(https.auto_redirect);
}

#[test]
fn test_site_builder_https_config() {
    let https = Https {
        auto_redirect: false,
        cert_file: Some("cert.pem".to_string()),
        key_file: Some("key.pem".to_string()),
        ca_file: Some("ca.pem".to_string()),
    };

    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .https(https)
        .build();

    assert!(site.https_config.is_some());
    let config = site.https_config.unwrap();
    assert!(!config.auto_redirect);
    assert_eq!(config.ca_file, Some("ca.pem".to_string()));
}

#[test]
fn test_site_builder_full_example() {
    let site = SiteBuilder::new("my-app")
        .domain("example.com")
        .domain("www.example.com")
        .root("./dist")
        .fallback_file("index.html")
        .default_index_file("index.html")
        .response_header("X-Frame-Options", "DENY")
        .redirect("/blog", "https://blog.example.com")
        .rewrite("/api/*", "/backend/$1")
        .build();

    assert_eq!(site.name, "my-app");
    assert_eq!(site.domain_names.len(), 2);
    assert_eq!(site.root, "./dist");
    assert_eq!(site.fallback_file, Some("index.html".to_string()));
    assert_eq!(site.response_headers.len(), 1);
    assert_eq!(site.redirects.len(), 1);
    assert_eq!(site.rewrites.len(), 1);
}

#[test]
fn test_site_add_certificate() {
    let mut site = SiteBuilder::new("my-site")
        .domain("example.com")
        .build();

    assert!(!site.has_certificate());
    assert!(site.https_config.is_none());

    site.add_certificate("./certs/cert.pem", "./certs/key.pem");

    assert!(site.has_certificate());
    assert!(site.https_config.is_some());

    let https = site.https_config.as_ref().unwrap();
    assert_eq!(https.cert_file, Some("./certs/cert.pem".to_string()));
    assert_eq!(https.key_file, Some("./certs/key.pem".to_string()));
    assert!(https.auto_redirect);
    assert!(https.ca_file.is_none());
}

#[test]
fn test_site_add_certificate_with_ca() {
    let mut site = SiteBuilder::new("my-site")
        .domain("example.com")
        .build();

    site.add_certificate_with_ca(
        "./certs/cert.pem",
        "./certs/key.pem",
        "./certs/ca.pem",
    );

    assert!(site.has_certificate());

    let https = site.https_config.as_ref().unwrap();
    assert_eq!(https.cert_file, Some("./certs/cert.pem".to_string()));
    assert_eq!(https.key_file, Some("./certs/key.pem".to_string()));
    assert_eq!(https.ca_file, Some("./certs/ca.pem".to_string()));
}

#[test]
fn test_site_remove_certificate() {
    let mut site = SiteBuilder::new("my-site")
        .domain("example.com")
        .manual_cert("./certs/cert.pem", "./certs/key.pem")
        .build();

    assert!(site.has_certificate());

    site.remove_certificate();

    assert!(!site.has_certificate());
    assert!(site.https_config.is_none());
}

#[test]
fn test_site_has_certificate_with_acme() {
    // Site with no https_config uses ACME (not manual cert)
    let site = SiteBuilder::new("my-site")
        .domain("example.com")
        .build();

    assert!(!site.has_certificate());
}

#[test]
fn test_site_add_certificate_overwrites_existing() {
    let mut site = SiteBuilder::new("my-site")
        .domain("example.com")
        .manual_cert("./old/cert.pem", "./old/key.pem")
        .build();

    site.add_certificate("./new/cert.pem", "./new/key.pem");

    let https = site.https_config.as_ref().unwrap();
    assert_eq!(https.cert_file, Some("./new/cert.pem".to_string()));
    assert_eq!(https.key_file, Some("./new/key.pem".to_string()));
}
