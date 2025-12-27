// Integration tests for file tools
// Tests use REAL filesystem — no mocks

use odincode::file_tools;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_file_read_existing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test_file.txt");
    let content = "Hello, World!";

    fs::write(&file_path, content).expect("Failed to write test file");

    let result = file_tools::file_read(&file_path);
    assert!(result.is_ok(), "file_read should succeed for existing file");
    assert_eq!(result.unwrap(), content);
}

#[test]
fn test_file_read_missing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("nonexistent.txt");

    let result = file_tools::file_read(&file_path);
    assert!(result.is_err(), "file_read should fail for missing file");
}

#[test]
fn test_file_write_new_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("new_file.txt");
    let content = "New content";

    let result = file_tools::file_write(&file_path, content);
    assert!(result.is_ok(), "file_write should succeed for new file");

    let read_content = fs::read_to_string(&file_path).expect("Failed to read back");
    assert_eq!(read_content, content);
}

#[test]
fn test_file_write_overwrite() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("overwrite.txt");
    fs::write(&file_path, "Old content").expect("Failed to write initial content");

    let new_content = "New content";
    let result = file_tools::file_write(&file_path, new_content);
    assert!(
        result.is_ok(),
        "file_write should succeed overwriting existing file"
    );

    let read_content = fs::read_to_string(&file_path).expect("Failed to read back");
    assert_eq!(read_content, new_content);
}

#[test]
fn test_file_write_missing_parent() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("nonexistent").join("file.txt");
    let content = "Content";

    let result = file_tools::file_write(&file_path, content);
    assert!(
        result.is_err(),
        "file_write should fail if parent directory missing"
    );
}

#[test]
fn test_file_create_new_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("create_new.txt");
    let content = "Created content";

    let result = file_tools::file_create(&file_path, content);
    assert!(result.is_ok(), "file_create should succeed for new file");

    let read_content = fs::read_to_string(&file_path).expect("Failed to read back");
    assert_eq!(read_content, content);
}

#[test]
fn test_file_create_existing_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("existing.txt");
    fs::write(&file_path, "Existing content").expect("Failed to write initial content");

    let content = "New content";
    let result = file_tools::file_create(&file_path, content);
    assert!(
        result.is_err(),
        "file_create should fail if file already exists"
    );

    // Original content should be unchanged
    let read_content = fs::read_to_string(&file_path).expect("Failed to read back");
    assert_eq!(read_content, "Existing content");
}

#[test]
fn test_file_create_creates_parent_dirs() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("nested").join("dirs").join("file.txt");
    let content = "Content in nested dir";

    let result = file_tools::file_create(&file_path, content);
    assert!(
        result.is_ok(),
        "file_create should create parent directories"
    );

    assert!(file_path.exists(), "File should exist after creation");
    let read_content = fs::read_to_string(&file_path).expect("Failed to read back");
    assert_eq!(read_content, content);
}

#[test]
fn test_file_search_pattern_match() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("search_test.txt");
    fs::write(&file_path, "line one\nline two\nline three\n").expect("Failed to write");

    let result = file_tools::file_search("two", temp_dir.path());
    assert!(result.is_ok(), "file_search should succeed");

    let matches = result.unwrap();
    assert_eq!(matches.len(), 1, "Should find one match");
    assert_eq!(matches[0].line_number, 2, "Match should be on line 2");
    assert!(
        matches[0].line.contains("two"),
        "Match should contain 'two'"
    );
}

#[test]
fn test_file_search_no_results() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("no_match.txt");
    fs::write(&file_path, "line one\nline two\n").expect("Failed to write");

    let result = file_tools::file_search("nonexistent", temp_dir.path());
    assert!(
        result.is_ok(),
        "file_search should succeed even with no results"
    );

    let matches = result.unwrap();
    assert_eq!(matches.len(), 0, "Should find no matches");
}

#[test]
fn test_file_search_invalid_regex() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let result = file_tools::file_search("[invalid(", temp_dir.path());
    assert!(
        result.is_err(),
        "file_search should fail with invalid regex"
    );
}

#[test]
fn test_file_glob_matches() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    fs::write(temp_dir.path().join("test1.txt"), "content1").expect("Failed to write");
    fs::write(temp_dir.path().join("test2.txt"), "content2").expect("Failed to write");
    fs::write(temp_dir.path().join("other.rs"), "content3").expect("Failed to write");

    let result = file_tools::file_glob("*.txt", temp_dir.path());
    assert!(result.is_ok(), "file_glob should succeed");

    let files = result.unwrap();
    assert_eq!(files.len(), 2, "Should find two .txt files");

    // Check deterministic ordering (sorted)
    let mut names: Vec<_> = files
        .iter()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(names[0], "test1.txt");
    assert_eq!(names[1], "test2.txt");
}

#[test]
fn test_file_glob_no_matches() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let result = file_tools::file_glob("*.nonexistent", temp_dir.path());
    assert!(result.is_ok(), "file_glob should succeed with no matches");

    let files = result.unwrap();
    assert_eq!(files.len(), 0, "Should find no files");
}
