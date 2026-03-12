mod helpers;

use helpers::*;
use tempfile::TempDir;

#[test]
fn init_creates_data_directory() {
    let dir = TempDir::new().unwrap();
    todos_cmd(dir.path()).args(["init"]).assert().success();
    assert!(dir.path().join(".todos/tasks.json").exists());
}

#[test]
fn init_creates_valid_json() {
    let dir = TempDir::new().unwrap();
    todos_cmd(dir.path()).args(["init"]).assert().success();
    let content = std::fs::read_to_string(dir.path().join(".todos/tasks.json")).unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(data["version"], 2);
    assert_eq!(data["tasks"], serde_json::json!([]));
}

#[test]
fn init_refuses_overwrite_without_force() {
    let dir = TempDir::new().unwrap();
    todos_cmd(dir.path()).args(["init"]).assert().success();
    todos_cmd(dir.path()).args(["init"]).assert().failure();
}

#[test]
fn init_force_overwrites() {
    let dir = TempDir::new().unwrap();
    todos_cmd(dir.path()).args(["init"]).assert().success();
    todos_cmd(dir.path())
        .args(["init", "--force"])
        .assert()
        .success();
}

#[test]
fn init_json_output() {
    let dir = TempDir::new().unwrap();
    let json = todos_json(dir.path(), &["init"]);
    assert_eq!(json["success"], true);
}
