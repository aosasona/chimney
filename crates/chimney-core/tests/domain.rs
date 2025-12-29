use chimney::config::{Domain, DomainIndex, WILDCARD_DOMAIN};

#[test]
fn test_domain_from_str() {
    let domain: Domain = "example.com".to_string().try_into().unwrap();
    assert_eq!(domain.name, "example.com");
    assert!(domain.port.is_none());

    let domain: Domain = "http://example.com:8080".to_string().try_into().unwrap();
    assert_eq!(domain.name, "example.com");
    assert_eq!(domain.port, Some(8080));

    let domain: Domain = "*".to_string().try_into().unwrap();
    assert_eq!(domain.name, WILDCARD_DOMAIN);
    assert!(domain.port.is_none());
}

#[test]
fn test_domain_index_insert_and_get() {
    let mut index = DomainIndex::default();
    let domain = Domain {
        name: "example.com".to_string(),
        port: Some(80),
    };
    index
        .insert(domain.clone(), "example_site".to_string())
        .unwrap();

    assert!(index.contains(&domain));
    assert_eq!(index.get(&domain), Some(&"example_site".to_string()));
}

#[test]
fn test_wildcard_index() {
    let mut index = DomainIndex::default();
    let wildcard_domain = Domain {
        name: WILDCARD_DOMAIN.to_string(),
        port: None,
    };
    index
        .insert(wildcard_domain.clone(), "wildcard_site".to_string())
        .unwrap();

    // This should return the wildcard site name
    let example_domain = Domain {
        name: "example.com".to_string(),
        port: None,
    };
    assert!(index.get(&example_domain).is_some());
    assert_eq!(index.get(&example_domain).unwrap(), "wildcard_site");
}

#[test]
fn test_domain_lookup_ignores_port() {
    let mut index = DomainIndex::default();

    // Site configured without port
    let domain_no_port = Domain {
        name: "localhost".to_string(),
        port: None,
    };
    index
        .insert(domain_no_port, "localhost_site".to_string())
        .unwrap();

    // Request comes in with port - should still match
    let domain_with_port = Domain {
        name: "localhost".to_string(),
        port: Some(8080),
    };
    assert_eq!(
        index.get(&domain_with_port),
        Some(&"localhost_site".to_string())
    );

    // Request without port should also match
    let domain_no_port = Domain {
        name: "localhost".to_string(),
        port: None,
    };
    assert_eq!(
        index.get(&domain_no_port),
        Some(&"localhost_site".to_string())
    );
}
