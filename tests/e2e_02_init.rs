mod helpers;

use helpers::*;
use tempfile::TempDir;

#[test]
fn init_creates_data_directory() {
    let dir = TempDir::new().unwrap();
    todos_cmd(dir.path()).args(["init"]).assert().success();
    assert!(dir.path().join(".todos/todos.db").exists());
}

#[test]
fn init_creates_valid_database() {
    let dir = TempDir::new().unwrap();
    todos_cmd(dir.path()).args(["init"]).assert().success();
    let db_path = dir.path().join(".todos/todos.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    // Verify tables exist
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(tables.contains(&"tasks".to_string()));
    assert!(tables.contains(&"archive".to_string()));
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
