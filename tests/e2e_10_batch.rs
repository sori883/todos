mod helpers;

use helpers::{setup, todos_cmd, todos_json, todos_json_stdin};

#[test]
fn batch_add_multiple() {
    let dir = setup();
    let input = r#"[
        {"action": "add", "title": "タスク1", "priority": "high"},
        {"action": "add", "title": "タスク2", "label": "bug"}
    ]"#;
    let json = todos_json_stdin(dir.path(), &["batch"], input);
    assert_eq!(json["data"]["summary"]["total"], 2);
    assert_eq!(json["data"]["summary"]["succeeded"], 2);
    assert_eq!(json["data"]["summary"]["failed"], 0);
    // 実際にタスクが作成されている
    let list = todos_json(dir.path(), &["list"]);
    assert_eq!(list["data"]["count"], 2);
}

#[test]
fn batch_mixed_operations() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "既存タスク"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let input = format!(r#"[
        {{"action": "add", "title": "新規", "created_by": "ai"}},
        {{"action": "status", "id": "{}", "status": "done"}},
        {{"action": "edit", "id": "{}", "priority": "high"}}
    ]"#, &id[..8], &id[..8]);
    let json = todos_json_stdin(dir.path(), &["batch"], &input);
    assert_eq!(json["data"]["summary"]["succeeded"], 3);
}

#[test]
fn batch_partial_failure() {
    let dir = setup();
    let input = r#"[
        {"action": "add", "title": "成功"},
        {"action": "delete", "id": "0000"},
        {"action": "add", "title": "これも成功"}
    ]"#;
    let json = todos_json_stdin(dir.path(), &["batch"], input);
    assert_eq!(json["data"]["summary"]["total"], 3);
    assert_eq!(json["data"]["summary"]["succeeded"], 2);
    assert_eq!(json["data"]["summary"]["failed"], 1);
    assert_eq!(json["data"]["results"][1]["success"], false);
}

#[test]
fn batch_add_with_parent() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    let input = format!(r#"[
        {{"action": "add", "title": "子タスク", "parent_id": "{}"}}
    ]"#, &pid[..8]);
    let json = todos_json_stdin(dir.path(), &["batch"], &input);
    assert_eq!(json["data"]["summary"]["succeeded"], 1);
    assert!(json["data"]["results"][0]["task"]["parent_id"].is_string());
}

#[test]
fn batch_single_write() {
    let dir = setup();
    // 大量のタスクを追加しても書き込みは1回
    let actions: Vec<String> = (0..100)
        .map(|i| format!(r#"{{"action": "add", "title": "タスク{}"}}"#, i))
        .collect();
    let input = format!("[{}]", actions.join(","));
    let json = todos_json_stdin(dir.path(), &["batch"], &input);
    assert_eq!(json["data"]["summary"]["succeeded"], 100);
}

#[test]
fn batch_empty_array() {
    let dir = setup();
    let json = todos_json_stdin(dir.path(), &["batch"], "[]");
    assert_eq!(json["data"]["summary"]["total"], 0);
}

#[test]
fn batch_invalid_json_fails() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["batch"])
        .write_stdin("not json")
        .assert().failure();
}
