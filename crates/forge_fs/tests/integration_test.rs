//! Integration tests for forge_fs file operations.
//!
//! These tests exercise the public API of `ForgeFS` through real filesystem
//! operations in isolated temporary directories.

use anyhow::Result;
use forge_fs::ForgeFS;
use tempfile::TempDir;

/// Helper: create an isolated temp directory and return it (keeps it alive).
fn temp_dir() -> Result<TempDir> {
    Ok(tempfile::tempdir()?)
}

// ---------------------------------------------------------------------------
// 1. File write + read round-trip
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_file_write_and_read() -> Result<()> {
    let dir = temp_dir()?;
    let file_path = dir.path().join("hello.txt");
    let content = b"Hello, ForgeFS!";

    // Write
    ForgeFS::write(&file_path, content).await?;

    // Read bytes
    let bytes = ForgeFS::read(&file_path).await?;
    assert_eq!(bytes, content, "raw bytes should match");

    // Read as string
    let text = ForgeFS::read_to_string(&file_path).await?;
    assert_eq!(text, "Hello, ForgeFS!", "string content should match");

    // Read UTF-8 with lossy conversion
    let utf8 = ForgeFS::read_utf8(&file_path).await?;
    assert_eq!(utf8, "Hello, ForgeFS!");

    Ok(())
}

// ---------------------------------------------------------------------------
// 2. File append
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_file_append() -> Result<()> {
    let dir = temp_dir()?;
    let file_path = dir.path().join("log.txt");

    ForgeFS::write(&file_path, b"line1").await?;
    ForgeFS::append(&file_path, b"\nline2").await?;
    ForgeFS::append(&file_path, b"\nline3").await?;

    let content = ForgeFS::read_to_string(&file_path).await?;
    assert_eq!(content, "line1\nline2\nline3");

    Ok(())
}

// ---------------------------------------------------------------------------
// 3. File delete
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_file_delete() -> Result<()> {
    let dir = temp_dir()?;
    let file_path = dir.path().join("to_delete.txt");

    ForgeFS::write(&file_path, b"delete me").await?;
    assert!(ForgeFS::exists(&file_path), "file should exist after write");
    assert!(ForgeFS::is_file(&file_path), "path should be a file");

    ForgeFS::remove_file(&file_path).await?;
    assert!(
        !ForgeFS::exists(&file_path),
        "file should not exist after deletion"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Directory listing (read_dir)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_directory_listing() -> Result<()> {
    let dir = temp_dir()?;
    let sub = dir.path().join("data");

    ForgeFS::create_dir_all(&sub).await?;
    ForgeFS::write(sub.join("a.txt"), b"aaa").await?;
    ForgeFS::write(sub.join("b.txt"), b"bbb").await?;
    ForgeFS::write(sub.join("c.txt"), b"ccc").await?;

    let mut entries = ForgeFS::read_dir(&sub).await?;
    let mut names: Vec<String> = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();

    assert_eq!(names, vec!["a.txt", "b.txt", "c.txt"]);
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Path resolution (exists / is_file / nested dirs)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_path_resolution() -> Result<()> {
    let dir = temp_dir()?;

    // Non-existent path
    let ghost = dir.path().join("does_not").join("exist.txt");
    assert!(!ForgeFS::exists(&ghost));
    assert!(!ForgeFS::is_file(&ghost));

    // Create nested dirs and a file
    let nested = dir.path().join("a").join("b").join("c");
    ForgeFS::create_dir_all(&nested).await?;
    let file = nested.join("deep.txt");
    ForgeFS::write(&file, b"deep").await?;

    assert!(ForgeFS::exists(&file));
    assert!(ForgeFS::is_file(&file));
    assert!(ForgeFS::exists(&nested));
    assert!(!ForgeFS::is_file(&nested), "directory is not a file");

    Ok(())
}

// ---------------------------------------------------------------------------
// 6. File size
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_file_size() -> Result<()> {
    let dir = temp_dir()?;
    let file_path = dir.path().join("sized.txt");
    let content = b"1234567890";

    ForgeFS::write(&file_path, content).await?;
    let size = ForgeFS::file_size(&file_path).await?;
    assert_eq!(size, content.len() as u64);

    Ok(())
}

// ---------------------------------------------------------------------------
// 7. Error on read of missing file
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_read_missing_file_errors() -> Result<()> {
    let dir = temp_dir()?;
    let ghost = dir.path().join("nope.txt");

    let result = ForgeFS::read(&ghost).await;
    assert!(result.is_err(), "reading a missing file should fail");

    let result = ForgeFS::read_to_string(&ghost).await;
    assert!(
        result.is_err(),
        "read_to_string on missing file should fail"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// 8. create_dir_all for nested paths
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_create_dir_all_nested() -> Result<()> {
    let dir = temp_dir()?;
    let deep = dir.path().join("x").join("y").join("z");

    ForgeFS::create_dir_all(&deep).await?;
    assert!(ForgeFS::exists(&deep));
    assert!(deep.is_dir());

    // Write a file inside the deeply nested dir
    let file = deep.join("file.txt");
    ForgeFS::write(&file, b"nested").await?;
    assert!(ForgeFS::exists(&file));

    Ok(())
}
