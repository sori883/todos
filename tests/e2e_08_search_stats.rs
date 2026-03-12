mod helpers;

use helpers::{setup, todos_cmd, todos_json};

#[test]
fn search_by_title() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "API実装"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "テスト追加"])
        .assert()
        .success();
    let json = todos_json(dir.path(), &["search", "API"]);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["tasks"][0]["title"], "API実装");
}

#[test]
fn search_by_content() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "タスク1", "-d", "認証機能の実装"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "タスク2", "-d", "テスト追加"])
        .assert()
        .success();
    let json = todos_json(dir.path(), &["search", "認証"]);
    assert_eq!(json["data"]["count"], 1);
}

#[test]
fn search_case_insensitive() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "API endpoint"])
        .assert()
        .success();
    let json = todos_json(dir.path(), &["search", "api"]);
    assert_eq!(json["data"]["count"], 1);
}

#[test]
fn search_with_filter() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "API実装", "-l", "feature"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "APIバグ修正", "-l", "bug"])
        .assert()
        .success();
    let json = todos_json(dir.path(), &["search", "API", "-l", "bug"]);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["tasks"][0]["label"], "bug");
}

#[test]
fn search_no_results() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "テスト"])
        .assert()
        .success();
    let json = todos_json(dir.path(), &["search", "存在しない"]);
    assert_eq!(json["data"]["count"], 0);
}

#[test]
fn search_is_alias_for_list_query() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "検索対象"])
        .assert()
        .success();
    let search = todos_json(dir.path(), &["search", "検索"]);
    let list = todos_json(dir.path(), &["list", "-q", "検索"]);
    assert_eq!(search["data"]["count"], list["data"]["count"]);
}

#[test]
fn stats_empty() {
    let dir = setup();
    let json = todos_json(dir.path(), &["stats"]);
    assert_eq!(json["data"]["total"], 0);
}

#[test]
fn stats_counts() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "A", "-p", "high", "-l", "bug", "-P", "svc-a"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "B", "-p", "low", "-l", "feature", "-P", "svc-a"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args([
            "add", "C", "-p", "high", "-l", "bug", "-P", "svc-b", "-c", "ai",
        ])
        .assert()
        .success();
    let json = todos_json(dir.path(), &["stats"]);
    assert_eq!(json["data"]["total"], 3);
    assert_eq!(json["data"]["by_status"]["todo"], 3);
    assert_eq!(json["data"]["by_priority"]["high"], 2);
    assert_eq!(json["data"]["by_priority"]["low"], 1);
    assert_eq!(json["data"]["by_label"]["bug"], 2);
    assert_eq!(json["data"]["by_label"]["feature"], 1);
    assert_eq!(json["data"]["by_project"]["svc-a"], 2);
    assert_eq!(json["data"]["by_project"]["svc-b"], 1);
    assert_eq!(json["data"]["by_creator"]["human"], 2);
    assert_eq!(json["data"]["by_creator"]["ai"], 1);
}

#[test]
fn stats_with_filter() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "A", "-P", "svc-a"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "B", "-P", "svc-b"])
        .assert()
        .success();
    let json = todos_json(dir.path(), &["stats", "-P", "svc-a"]);
    assert_eq!(json["data"]["total"], 1);
}
