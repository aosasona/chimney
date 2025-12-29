use chimney::filesystem::{local::LocalFS, mock::MockFilesystem, Filesystem};

// Local filesystem tests
#[test]
fn test_local_fs_new() {
    let temp_dir = tempfile::tempdir().unwrap();
    let fs = LocalFS::new(temp_dir.path().to_path_buf()).unwrap();
    assert!(fs.path().exists());
}

#[test]
fn test_local_fs_read_dir() {
    let temp_dir = tempfile::tempdir().unwrap();
    let fs = LocalFS::new(temp_dir.path().to_path_buf()).unwrap();
    fs.read_dir(temp_dir.path().to_path_buf()).unwrap();
}

#[test]
fn test_local_fs_list_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let fs = LocalFS::new(temp_dir.path().to_path_buf()).unwrap();
    let files = fs.list_files(temp_dir.path().to_path_buf()).unwrap();
    assert!(files.is_empty());
}

// Mock filesystem tests
#[test]
fn test_mock_filesystem_read_dir() {
    let fs = MockFilesystem;
    let path = std::path::PathBuf::from("public");
    let files = fs.read_dir(path).unwrap();

    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.path.ends_with("style.css")));
    assert!(files.iter().any(|f| f.path.ends_with("script.js")));
}

#[test]
fn test_mock_filesystem_list_files() {
    let fs = MockFilesystem;
    let path = std::path::PathBuf::from("data");
    let files = fs.list_files(path).unwrap();

    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.ends_with("example.json")));
    assert!(files.iter().any(|f| f.ends_with("note.txt")));
}

#[test]
fn test_mock_filesystem_read_file() {
    let fs = MockFilesystem;
    let path = std::path::PathBuf::from("index.html");
    let content = fs.read_file(path).unwrap();
    let content_html = String::from_utf8(content.bytes().to_vec()).unwrap();

    assert_eq!(
        content_html,
        "<html><body><h1>Welcome to Chimney!</h1></body></html>"
    );
}

#[test]
fn test_mock_filesystem_get_file_metadata() {
    let fs = MockFilesystem;
    let path = std::path::PathBuf::from("about.html");
    let file = fs.stat(path).unwrap();

    assert!(file.is_file());
    assert_eq!(file.path.to_string_lossy(), "about.html");
}

#[test]
fn test_mock_filesystem_file_not_found() {
    let fs = MockFilesystem;
    let path = std::path::PathBuf::from("nonexistent.txt");
    let result = fs.read_file(path);
    assert!(result.is_err());
    if let Err(chimney::filesystem::FilesystemError::ReadFileError { path, message }) = result {
        assert_eq!(path.to_string_lossy(), "nonexistent.txt");
        assert_eq!(message, "File not found in mock filesystem");
    } else {
        panic!("Expected ReadFileError");
    }
}

#[test]
fn test_mock_filesystem_metadata_not_found() {
    let fs = MockFilesystem;
    let path = std::path::PathBuf::from("nonexistent.txt");
    let result = fs.stat(path);
    assert!(result.is_err());
    if let Err(chimney::filesystem::FilesystemError::MetadataError { path, message }) = result {
        assert_eq!(path.to_string_lossy(), "nonexistent.txt");
        assert_eq!(message, "File not found in mock filesystem");
    } else {
        panic!("Expected MetadataError");
    }
}

#[test]
fn test_mock_filesystem_exists() {
    let fs = MockFilesystem;
    let path = std::path::PathBuf::from("index.html");
    let exists = fs.exists(path).unwrap();
    assert!(exists);

    let non_existent_path = std::path::PathBuf::from("nonexistent.txt");
    let exists = fs.exists(non_existent_path).unwrap();
    assert!(!exists);
}
