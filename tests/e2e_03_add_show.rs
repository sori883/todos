mod helpers;

use helpers::*;

// === add command ===

#[test]
fn add_basic_task() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "テストタスク"])
        .assert()
        .success();
}

#[test]
fn add_with_all_options() {
    let dir = setup();
    todos_cmd(dir.path())
        .args([
            "add",
            "API実装",
            "-d",
            "REST APIを実装する",
            "-p",
            "high",
            "-c",
            "ai",
            "-l",
            "feature",
            "-P",
            "user-service",
        ])
        .assert()
        .success();
}

#[test]
fn add_json_output_contains_task() {
    let dir = setup();
    let json = todos_json(dir.path(), &["add", "テスト"]);
    assert_eq!(json["success"], true);
    assert!(json["data"]["task"]["id"].is_string());
    assert_eq!(json["data"]["task"]["title"], "テスト");
    assert_eq!(json["data"]["task"]["status"], "todo");
    assert_eq!(json["data"]["task"]["priority"], "none");
    assert_eq!(json["data"]["task"]["created_by"], "human");
}

#[test]
fn add_with_priority() {
    let dir = setup();
    let json = todos_json(dir.path(), &["add", "緊急", "-p", "critical"]);
    assert_eq!(json["data"]["task"]["priority"], "critical");
}

#[test]
fn add_invalid_label_fails() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "テスト", "-l", "unknown_label"])
        .assert()
        .failure();
}

#[test]
fn add_subtask() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親タスク"]);
    let parent_id = parent["data"]["task"]["id"].as_str().unwrap();
    let child = todos_json(dir.path(), &["add", "子タスク", "--parent", &parent_id[..8]]);
    assert_eq!(child["data"]["task"]["parent_id"], parent_id);
}

#[test]
fn add_subtask_inherits_project() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親", "-P", "user-service"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    let child = todos_json(dir.path(), &["add", "子", "--parent", &pid[..8]]);
    assert_eq!(child["data"]["task"]["project"], "user-service");
}

#[test]
fn add_subtask_of_subtask_fails() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    let child = todos_json(dir.path(), &["add", "子", "--parent", &pid[..8]]);
    let cid = child["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["add", "孫", "--parent", &cid[..8]])
        .assert()
        .failure();
}

#[test]
fn add_subtask_with_recurrence_fails() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["add", "子", "--parent", &pid[..8], "--recurrence", "daily"])
        .assert()
        .failure();
}

// === show command ===

#[test]
fn show_task_by_prefix() {
    let dir = setup();
    let json = todos_json(dir.path(), &["add", "テスト", "-p", "high"]);
    let id = json["data"]["task"]["id"].as_str().unwrap();
    let show = todos_json(dir.path(), &["show", &id[..8]]);
    assert_eq!(show["data"]["task"]["title"], "テスト");
    assert_eq!(show["data"]["task"]["priority"], "high");
}

#[test]
fn show_not_found() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["show", "0000"])
        .assert()
        .failure();
}

#[test]
fn show_prefix_too_short() {
    let dir = setup();
    todos_json(dir.path(), &["add", "テスト"]);
    todos_cmd(dir.path())
        .args(["show", "ab"])
        .assert()
        .failure();
}

#[test]
fn show_parent_includes_subtasks() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_json(dir.path(), &["add", "子1", "--parent", &pid[..8]]);
    todos_json(dir.path(), &["add", "子2", "--parent", &pid[..8]]);
    let show = todos_json(dir.path(), &["show", &pid[..8]]);
    assert_eq!(show["data"]["subtasks"].as_array().unwrap().len(), 2);
}

#[test]
fn show_subtask_includes_parent_info() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親タスク"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    let child = todos_json(dir.path(), &["add", "子", "--parent", &pid[..8]]);
    let cid = child["data"]["task"]["id"].as_str().unwrap();
    let show = todos_json(dir.path(), &["show", &cid[..8]]);
    assert!(show["data"]["parent"].is_object());
    assert_eq!(show["data"]["parent"]["title"], "親タスク");
}
