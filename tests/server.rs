use std::collections::HashMap;
use std::path::PathBuf;

use chimney::config::*;
use chimney::server::*;

fn mock_server() -> Server {
    let rewrites = {
        let mut rewrites = HashMap::new();
        rewrites.insert(
            "/home".to_string(),
            Rewrite::Config {
                to: "/index.html".to_string(),
            },
        );
        rewrites.insert(
            "/page-2".to_string(),
            Rewrite::Target("another_page.html".to_string()),
        );
        rewrites
    };

    let redirects = {
        let mut redirects: HashMap<String, Redirect> = HashMap::new();
        redirects.insert(
            "/twitch".to_string(),
            Redirect::Target("https://twitch.tv".to_string()),
        );
        redirects.insert(
            "/google".to_string(),
            Redirect::Config {
                to: "https://google.com".to_string(),
                replay: true,
            },
        );
        redirects
    };

    let config = Config {
        host: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        port: 80,
        enable_logging: true,
        mode: Some(Mode::Single),
        root_dir: "./examples/basic/public".to_string(),
        fallback_document: Some("fallback.html".to_string()),
        domain_names: vec![],
        https: None,
        headers: HashMap::new(),
        rewrites,
        redirects,
    };

    Server::new(config)
}

#[test]
pub fn find_rewrite_or_test() {
    let mut server = mock_server();

    // This is a valid rewrite so it should return the new path
    assert_eq!(server.find_rewrite_or("/home"), "/index.html".to_string());

    // This is a valid rewrite so it should return the new path
    assert_eq!(
        server.find_rewrite_or("/page-2"),
        "/another_page.html".to_string()
    );

    // This is not a valid rewrite so it should return the original path
    assert_eq!(server.find_rewrite_or("/not-found"), "/not-found");

    // This is not a valid rewrite so it should return the original path
    assert_eq!(server.find_rewrite_or("/"), "/".to_string());

    // Now it is a valid rewrite so it should return the new path
    server.config.rewrites.insert(
        "/".to_string(),
        Rewrite::Config {
            to: "/index_rewrite.html".to_string(),
        },
    );
    assert_eq!(
        server.find_rewrite_or("/"),
        "/index_rewrite.html".to_string()
    );
}

#[test]
pub fn get_file_path_test() {
    let mut server = mock_server();

    // This is a valid file so it should return the path to the file
    assert_eq!(
        server.get_valid_file_path("/index.html"),
        Some(PathBuf::from(format!(
            "{}/index.html",
            server.config.root_dir
        )))
    );

    // the fallback path doesn't exist, the file doesn't exist, and the directory doesn't
    // exist, so we should get back None
    assert_eq!(server.get_valid_file_path("/not-found"), None);

    // this is a valid fallback document so it should return the path to the fallback in this
    // case
    server.config.fallback_document = Some("another_page.html".to_string());
    assert_eq!(
        server.get_valid_file_path("/not-found"),
        Some(PathBuf::from(format!(
            "{}/{}",
            server.config.root_dir,
            server.config.fallback_document.clone().unwrap()
        )))
    );

    // The has no root html file but since it is a directory and it has an index.html file,
    // it should return the path to the index.html file
    server.config.root_dir = "./examples/basic".to_string();
    assert_eq!(
        server.get_valid_file_path("/public"),
        Some(PathBuf::from(format!(
            "{}/public/index.html",
            server.config.root_dir
        )))
    );

    server.config.root_dir = "./examples/trulyao/blog/arguments".to_string();
    assert_eq!(
        server.get_valid_file_path("/"),
        Some(PathBuf::from(format!(
            "{}/index.html",
            server.config.root_dir
        )))
    );

    // This directory has no index.html file so it should return None
    server.config.root_dir = "./examples/trulyao/images".to_string();
    assert_eq!(server.get_valid_file_path("/"), None);
}

#[test]
pub fn find_redirect_test() {
    let mut server = mock_server();

    // This is a valid redirect so it should return the new path
    assert_eq!(
        server.find_redirect("/twitch"),
        Some(("https://twitch.tv".to_string(), false))
    );

    // This is a valid redirect so it should return the new path
    assert_eq!(
        server.find_redirect("/google"),
        Some(("https://google.com".to_string(), true))
    );

    // This is not a valid redirect so it should return None
    assert_eq!(server.find_redirect("/not-found"), None);

    // Test for `/`
    assert_eq!(server.find_redirect("/"), None);

    // Now it is a valid redirect so it should return the new path
    server.config.redirects.insert(
        "/".to_string(),
        Redirect::Target("https://example.com".to_string()),
    );

    assert_eq!(
        server.find_redirect("/"),
        Some(("https://example.com".to_string(), false))
    );
}
