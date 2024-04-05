use std::collections::HashMap;
use std::path::PathBuf;

use chimney::config::*;
use chimney::server::*;

fn mock_server() -> (Server, Config) {
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
        mode: Mode::Single,
        root: Root::Path("./examples/basic/public".to_string()),
        fallback_document: Some("fallback.html".to_string()),
        domain_names: vec![],
        https: None,
        headers: HashMap::new(),
        rewrites,
        redirects,
    };

    let mut server = Server::new(&Opts {
        host: config.host,
        port: config.port,
        enable_logging: config.enable_logging,
        mode: config.mode.clone(),
        root_dir: config.root.clone().into(),
    });
    server.register("default".to_string(), &config);

    return (server, config);
}

#[test]
pub fn find_rewrite_or_test() {
    let (server, config) = mock_server();

    // This is a valid rewrite so it should return the new path
    assert_eq!(
        server.find_rewrite_or(&config, "/home"),
        "/index.html".to_string()
    );

    // This is a valid rewrite so it should return the new path
    assert_eq!(
        server.find_rewrite_or(&config, "/page-2"),
        "/another_page.html".to_string()
    );

    // This is not a valid rewrite so it should return the original path
    assert_eq!(server.find_rewrite_or(&config, "/not-found"), "/not-found");

    // This is not a valid rewrite so it should return the original path
    assert_eq!(server.find_rewrite_or(&config, "/"), "/".to_string());

    // Now it is a valid rewrite so it should return the new path
    let mut config = config.clone();

    config.rewrites.insert(
        "/".to_string(),
        Rewrite::Config {
            to: "/index_rewrite.html".to_string(),
        },
    );
    assert_eq!(
        server.find_rewrite_or(&config, "/"),
        "/index_rewrite.html".to_string()
    );
}

#[test]
pub fn get_file_path_test() {
    let (server, config) = mock_server();

    // This is a valid file so it should return the path to the file
    assert_eq!(
        server.get_valid_file_path(&config, "/index.html"),
        Some(PathBuf::from(format!("{}/index.html", config.root)))
    );

    // the fallback path doesn't exist, the file doesn't exist, and the directory doesn't
    // exist, so we should get back None
    assert_eq!(server.get_valid_file_path(&config, "/not-found"), None);

    // this is a valid fallback document so it should return the path to the fallback in this
    // case
    let mut config = config.clone();
    config.fallback_document = Some("another_page.html".to_string());
    assert_eq!(
        server.get_valid_file_path(&config, "/not-found"),
        Some(PathBuf::from(format!(
            "{}/{}",
            config.root,
            config.fallback_document.clone().unwrap()
        )))
    );

    // The has no root html file but since it is a directory and it has an index.html file,
    // it should return the path to the index.html file
    config.root = "./examples/basic".to_string().into();
    assert_eq!(
        server.get_valid_file_path(&config, "/public"),
        Some(PathBuf::from(format!("{}/public/index.html", config.root)))
    );

    config.root = "./examples/trulyao/blog/arguments".to_string().into();
    assert_eq!(
        server.get_valid_file_path(&config, "/"),
        Some(PathBuf::from(format!("{}/index.html", config.root)))
    );

    // This directory has no index.html file so it should return None
    config.root = "./examples/trulyao/images".to_string().into();
    assert_eq!(server.get_valid_file_path(&config, "/"), None);
}

#[test]
pub fn find_redirect_test() {
    let (server, config) = mock_server();

    // This is a valid redirect so it should return the new path
    assert_eq!(
        server.find_redirect(&config, "/twitch"),
        Some(("https://twitch.tv".to_string(), false))
    );

    // This is a valid redirect so it should return the new path
    assert_eq!(
        server.find_redirect(&config, "/google"),
        Some(("https://google.com".to_string(), true))
    );

    // This is not a valid redirect so it should return None
    assert_eq!(server.find_redirect(&config, "/not-found"), None);

    // Test for `/`
    assert_eq!(server.find_redirect(&config, "/"), None);

    // Now it is a valid redirect so it should return the new path
    let mut config = config.clone();
    config.redirects.insert(
        "/".to_string(),
        Redirect::Target("https://example.com".to_string()),
    );

    assert_eq!(
        server.find_redirect(&config, "/"),
        Some(("https://example.com".to_string(), false))
    );
}
