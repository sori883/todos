# 04: list コマンド

## 概要

タスク一覧表示。フィルタ、ソート、ツリー表示、フラット表示を実装。

## 依存タスク

03-add-show

## 先に書く E2E テスト

```rust
#[test]
fn list_empty() {
    let dir = setup();
    let json = todos_json(dir.path(), &["list"]);
    assert_eq!(json["data"]["count"], 0);
    assert_eq!(json["data"]["tasks"].as_array().unwrap().len(), 0);
}

#[test]
fn list_shows_added_tasks() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "タスク1"]).assert().success();
    todos_cmd(dir.path()).args(["add", "タスク2"]).assert().success();
    let json = todos_json(dir.path(), &["list"]);
    assert_eq!(json["data"]["count"], 2);
}

#[test]
fn list_filter_by_status() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "タスク1"]).assert().success();
    todos_cmd(dir.path()).args(["add", "タスク2"]).assert().success();
    // デフォルトは todo + in_progress のみ
    let json = todos_json(dir.path(), &["list"]);
    assert_eq!(json["data"]["count"], 2);
    // -s で絞り込み
    let json = todos_json(dir.path(), &["list", "-s", "in_progress"]);
    assert_eq!(json["data"]["count"], 0);
}

#[test]
fn list_filter_by_priority() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "低", "-p", "low"]).assert().success();
    todos_cmd(dir.path()).args(["add", "高", "-p", "high"]).assert().success();
    let json = todos_json(dir.path(), &["list", "-p", "high"]);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["tasks"][0]["title"], "高");
}

#[test]
fn list_filter_by_label() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "バグ", "-l", "bug"]).assert().success();
    todos_cmd(dir.path()).args(["add", "機能", "-l", "feature"]).assert().success();
    let json = todos_json(dir.path(), &["list", "-l", "bug"]);
    assert_eq!(json["data"]["count"], 1);
}

#[test]
fn list_filter_by_project() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "A", "-P", "svc-a"]).assert().success();
    todos_cmd(dir.path()).args(["add", "B", "-P", "svc-b"]).assert().success();
    let json = todos_json(dir.path(), &["list", "-P", "svc-a"]);
    assert_eq!(json["data"]["count"], 1);
}

#[test]
fn list_filter_by_created_by() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "人間", "-c", "human"]).assert().success();
    todos_cmd(dir.path()).args(["add", "AI", "-c", "ai"]).assert().success();
    let json = todos_json(dir.path(), &["list", "-c", "ai"]);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["tasks"][0]["title"], "AI");
}

#[test]
fn list_multiple_filters() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "A", "-p", "high", "-l", "bug"]).assert().success();
    todos_cmd(dir.path()).args(["add", "B", "-p", "low", "-l", "bug"]).assert().success();
    todos_cmd(dir.path()).args(["add", "C", "-p", "high", "-l", "feature"]).assert().success();
    let json = todos_json(dir.path(), &["list", "-p", "high", "-l", "bug"]);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["tasks"][0]["title"], "A");
}

#[test]
fn list_sort_by_priority() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "低", "-p", "low"]).assert().success();
    todos_cmd(dir.path()).args(["add", "高", "-p", "high"]).assert().success();
    let json = todos_json(dir.path(), &["list", "--sort", "priority"]);
    assert_eq!(json["data"]["tasks"][0]["title"], "高");
}

#[test]
fn list_limit() {
    let dir = setup();
    for i in 0..5 {
        todos_cmd(dir.path()).args(["add", &format!("タスク{i}")]).assert().success();
    }
    let json = todos_json(dir.path(), &["list", "--limit", "3"]);
    assert_eq!(json["data"]["tasks"].as_array().unwrap().len(), 3);
}

#[test]
fn list_tree_display() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path()).args(["add", "子", "--parent", &pid[..8]]).assert().success();
    // ツリー表示（デフォルト）: 親の直後に子
    let json = todos_json(dir.path(), &["list"]);
    assert_eq!(json["data"]["count"], 2);
    assert_eq!(json["data"]["tasks"][0]["title"], "親");
    assert_eq!(json["data"]["tasks"][1]["title"], "子");
    assert!(json["data"]["tasks"][1]["parent_id"].is_string());
}

#[test]
fn list_flat_display() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親", "-p", "low"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path()).args(["add", "子", "--parent", &pid[..8], "-p", "high"]).assert().success();
    // --flat でソート順に従う
    let json = todos_json(dir.path(), &["list", "--flat", "--sort", "priority"]);
    assert_eq!(json["data"]["tasks"][0]["title"], "子"); // high が先
}

#[test]
fn list_all_includes_done() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "タスク"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path()).args(["status", &id[..8], "done"]).assert().success();
    // デフォルト: done は非表示
    let json = todos_json(dir.path(), &["list"]);
    assert_eq!(json["data"]["count"], 0);
    // --all で表示
    let json = todos_json(dir.path(), &["list", "--all"]);
    assert_eq!(json["data"]["count"], 1);
}

#[test]
fn list_text_output_has_header() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "テスト"]).assert().success();
    todos_cmd(dir.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("ID"));
}
```

## 実装内容

### 1. service/task_service.rs

- `list_tasks()` -- フィルタ適用、ソート、default_view.show_completed 適用
- `list_tasks_tree()` -- 親子グルーピング、ツリー構造を返す
- `search_tasks()` -- title/description に対する部分一致

### 2. store/json_store.rs

- `list()` のフィルタ実装（status, priority, created_by, label, project, parent_id）
- ソート実装

### 3. cli/list.rs

- clap オプション定義（-s, -p, -c, -l, -P, -q, --sort, --reverse, --limit, --all, --flat）
- text 出力: ヘッダー行 + ツリー表示（サブタスクをインデント）
- json 出力: tasks 配列 + count

## 完了条件

- [ ] E2E テストが全て通る
- [ ] 全フィルタオプションが正しく動作する
- [ ] ソートが正しく動作する（priority, created_at, updated_at, title）
- [ ] ツリー表示で親の直後に子がインデント表示される
- [ ] `--flat` でフラット表示に切り替わる
- [ ] `--all` で done/cancelled を含む
- [ ] `--limit` で件数制限
- [ ] text 出力にヘッダー行がある
