mod helpers;

use helpers::*;

#[test]
fn edit_title() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "旧タイトル"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let edited = todos_json(dir.path(), &["edit", &id[..8], "-T", "新タイトル"]);
    assert_eq!(edited["data"]["task"]["title"], "新タイトル");
}

#[test]
fn edit_multiple_fields() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let edited = todos_json(
        dir.path(),
        &["edit", &id[..8], "-T", "更新", "-p", "high", "-l", "bug"],
    );
    assert_eq!(edited["data"]["task"]["title"], "更新");
    assert_eq!(edited["data"]["task"]["priority"], "high");
    assert_eq!(edited["data"]["task"]["label"], "bug");
}

#[test]
fn edit_updates_updated_at() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let original_updated = t["data"]["task"]["updated_at"]
        .as_str()
        .unwrap()
        .to_string();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let edited = todos_json(dir.path(), &["edit", &id[..8], "-T", "変更"]);
    assert_ne!(
        edited["data"]["task"]["updated_at"].as_str().unwrap(),
        original_updated
    );
}

#[test]
fn edit_no_options_fails() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["edit", &id[..8]])
        .assert()
        .failure();
}

#[test]
fn edit_invalid_label_fails() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["edit", &id[..8], "-l", "invalid"])
        .assert()
        .failure();
}

#[test]
fn edit_parent_to_make_subtask() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    let child = todos_json(dir.path(), &["add", "独立タスク"]);
    let cid = child["data"]["task"]["id"].as_str().unwrap();
    let edited = todos_json(dir.path(), &["edit", &cid[..8], "--parent", &pid[..8]]);
    assert_eq!(edited["data"]["task"]["parent_id"], pid);
}

#[test]
fn edit_parent_none_removes_parent() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    let child = todos_json(dir.path(), &["add", "子", "--parent", &pid[..8]]);
    let cid = child["data"]["task"]["id"].as_str().unwrap();
    let edited = todos_json(dir.path(), &["edit", &cid[..8], "--parent", "none"]);
    assert!(edited["data"]["task"]["parent_id"].is_null());
}

#[test]
fn edit_not_found() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["edit", "0000", "-T", "x"])
        .assert()
        .failure();
}
