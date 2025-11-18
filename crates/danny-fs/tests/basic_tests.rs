//! Basic tests for FileSystem implementations.

use danny_fs::{FileSystem, NativeFileSystem, DiscoveryOptions};
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn test_native_read_write() {
    let temp_dir = TempDir::new().unwrap();
    let fs = NativeFileSystem::new(temp_dir.path()).unwrap();

    let test_file = temp_dir.path().join("test.txt");
    let contents = "Hello, World!";

    // Write file
    fs.write(&test_file, contents).await.unwrap();

    // Read file
    let read_contents = fs.read_to_string(&test_file).await.unwrap();
    assert_eq!(read_contents, contents);
}

#[tokio::test]
async fn test_native_exists() {
    let temp_dir = TempDir::new().unwrap();
    let fs = NativeFileSystem::new(temp_dir.path()).unwrap();

    let test_file = temp_dir.path().join("test.txt");

    // File doesn't exist yet
    assert!(!fs.exists(&test_file).await.unwrap());

    // Create file
    fs.write(&test_file, "test").await.unwrap();

    // File exists now
    assert!(fs.exists(&test_file).await.unwrap());
}

#[tokio::test]
async fn test_native_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let fs = NativeFileSystem::new(temp_dir.path()).unwrap();

    let test_file = temp_dir.path().join("test.txt");
    let contents = "Hello, World!";
    fs.write(&test_file, contents).await.unwrap();

    let metadata = fs.metadata(&test_file).await.unwrap();
    assert!(metadata.exists);
    assert!(metadata.is_file);
    assert!(!metadata.is_dir);
    assert_eq!(metadata.size, contents.len() as u64);
}

#[tokio::test]
async fn test_native_discover_files() {
    let temp_dir = TempDir::new().unwrap();
    let fs = NativeFileSystem::new(temp_dir.path()).unwrap();

    // Create test files
    fs::write(temp_dir.path().join("foo.ts"), "test").unwrap();
    fs::write(temp_dir.path().join("bar.js"), "test").unwrap();
    fs::write(temp_dir.path().join("baz.txt"), "test").unwrap();

    let discovered = fs.discover_files(
        temp_dir.path(),
        &[".ts", ".js"],
        &[],
        &DiscoveryOptions::default(),
    ).await.unwrap();

    assert_eq!(discovered.len(), 2);
    assert!(discovered.iter().any(|p| p.ends_with("foo.ts")));
    assert!(discovered.iter().any(|p| p.ends_with("bar.js")));
    assert!(!discovered.iter().any(|p| p.ends_with("baz.txt")));
}

#[tokio::test]
async fn test_native_path_traversal_blocked() {
    let temp_dir = TempDir::new().unwrap();
    let fs = NativeFileSystem::new(temp_dir.path()).unwrap();

    let malicious_paths = vec![
        "../../../etc/passwd",
        "../../.ssh/id_rsa",
        "foo/../../bar/../../baz",
    ];

    for path in malicious_paths {
        let result = fs.read_to_string(std::path::Path::new(path)).await;
        assert!(result.is_err(), "Path traversal not blocked: {}", path);

        if let Err(e) = result {
            assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
        }
    }
}

#[tokio::test]
async fn test_native_remove_file() {
    let temp_dir = TempDir::new().unwrap();
    let fs = NativeFileSystem::new(temp_dir.path()).unwrap();

    let test_file = temp_dir.path().join("test.txt");
    fs.write(&test_file, "test").await.unwrap();
    assert!(fs.exists(&test_file).await.unwrap());

    fs.remove_file(&test_file).await.unwrap();
    assert!(!fs.exists(&test_file).await.unwrap());
}

#[cfg(feature = "wasm")]
mod wasm_tests {
    use super::*;
    use danny_fs::WasmFileSystem;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_wasm_read_write() {
        let mut files = HashMap::new();
        files.insert(PathBuf::from("/project/test.txt"), b"Hello, WASM!".to_vec());

        let fs = WasmFileSystem::new("/project", files).unwrap();

        let contents = fs.read_to_string(std::path::Path::new("/project/test.txt")).await.unwrap();
        assert_eq!(contents, "Hello, WASM!");
    }

    #[tokio::test]
    async fn test_wasm_exists() {
        let mut files = HashMap::new();
        files.insert(PathBuf::from("/project/test.txt"), b"test".to_vec());

        let fs = WasmFileSystem::new("/project", files).unwrap();

        assert!(fs.exists(std::path::Path::new("/project/test.txt")).await.unwrap());
        assert!(!fs.exists(std::path::Path::new("/project/missing.txt")).await.unwrap());
    }

    #[tokio::test]
    async fn test_wasm_discover_files() {
        let mut files = HashMap::new();
        files.insert(PathBuf::from("/project/foo.ts"), b"test".to_vec());
        files.insert(PathBuf::from("/project/bar.js"), b"test".to_vec());
        files.insert(PathBuf::from("/project/baz.txt"), b"test".to_vec());

        let fs = WasmFileSystem::new("/project", files).unwrap();

        let discovered = fs.discover_files(
            std::path::Path::new("/project"),
            &[".ts", ".js"],
            &[],
            &DiscoveryOptions::default(),
        ).await.unwrap();

        assert_eq!(discovered.len(), 2);
    }

    #[tokio::test]
    async fn test_wasm_path_traversal_blocked() {
        let files = HashMap::new();
        let fs = WasmFileSystem::new("/project", files).unwrap();

        let malicious_paths = vec![
            "../../../etc/passwd",
            "../../.ssh/id_rsa",
        ];

        for path in malicious_paths {
            let result = fs.read_to_string(std::path::Path::new(path)).await;
            assert!(result.is_err(), "Path traversal not blocked: {}", path);

            if let Err(e) = result {
                assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
            }
        }
    }
}

