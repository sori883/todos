mod helpers;

use helpers::*;

// === Basic status changes ===

#[test]
fn status_change_to_done() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "done"]);
    assert_eq!(result["data"]["task"]["status"], "done");
    assert!(result["data"]["task"]["completed_at"].is_string());
}

#[test]
fn status_change_to_in_progress() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "in_progress"]);
    assert_eq!(result["data"]["task"]["status"], "in_progress");
}

#[test]
fn status_change_to_cancelled() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "cancelled"]);
    assert_eq!(result["data"]["task"]["status"], "cancelled");
}

#[test]
fn status_done_sets_completed_at() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_json(dir.path(), &["status", &id[..8], "done"]);
    let show = todos_json(dir.path(), &["show", &id[..8]]);
    assert!(show["data"]["task"]["completed_at"].is_string());
}

#[test]
fn status_revert_from_done_clears_completed_at() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["status", &id[..8], "done"])
        .assert()
        .success();
    let result = todos_json(dir.path(), &["status", &id[..8], "todo"]);
    assert!(result["data"]["task"]["completed_at"].is_null());
}
