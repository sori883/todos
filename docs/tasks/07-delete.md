# 07: delete コマンド

## 概要

タスク削除。確認プロンプト、親タスク削除時の子タスク連動削除。

## 依存タスク

03-add-show

## 先に書く E2E テスト

```rust
#[test]
fn delete_task_with_yes() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "削除対象"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["delete", &id[..8], "--yes"])
        .assert().success();
    let list = todos_json(dir.path(), &["list", "--all"]);
    assert_eq!(list["data"]["count"], 0);
}

#[test]
fn delete_json_output() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "削除対象"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json_yes(dir.path(), &["delete", &id[..8]]);
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["task"]["title"], "削除対象");
}

#[test]
fn delete_not_found() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["delete", "0000", "--yes"])
        .assert().failure();
}

#[test]
fn delete_parent_cascades_to_children() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path()).args(["add", "子1", "--parent", &pid[..8]]).assert().success();
    todos_cmd(dir.path()).args(["add", "子2", "--parent", &pid[..8]]).assert().success();
    todos_cmd(dir.path()).args(["add", "他のタスク"]).assert().success();
    // 親を削除
    let result = todos_json_yes(dir.path(), &["delete", &pid[..8]]);
    assert_eq!(result["success"], true);
    // 子も削除されている
    let list = todos_json(dir.path(), &["list"]);
    assert_eq!(list["data"]["count"], 1);
    assert_eq!(list["data"]["tasks"][0]["title"], "他のタスク");
}

#[test]
fn delete_parent_json_shows_deleted_count() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path()).args(["add", "子1", "--parent", &pid[..8]]).assert().success();
    todos_cmd(dir.path()).args(["add", "子2", "--parent", &pid[..8]]).assert().success();
    let result = todos_json_yes(dir.path(), &["delete", &pid[..8]]);
    assert_eq!(result["data"]["deleted_subtasks"], 2);
}

#[test]
fn delete_child_only() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    let child = todos_json(dir.path(), &["add", "子", "--parent", &pid[..8]]);
    let cid = child["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["delete", &cid[..8], "--yes"])
        .assert().success();
    // 親はまだ存在
    let show = todos_json(dir.path(), &["show", &pid[..8]]);
    assert_eq!(show["data"]["task"]["title"], "親");
    assert_eq!(show["data"]["subtasks"].as_array().unwrap().len(), 0);
}

#[test]
fn delete_alias_rm() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["rm", &id[..8], "--yes"])
        .assert().success();
}
```

## 実装内容

### 1. service/task_service.rs

- `delete_task()` -- 子タスク検出、連動削除
- `DeleteResult` 構造体（削除されたタスク + 削除された子タスク数）

### 2. cli/delete.rs

- clap 引数定義（ID）
- 確認プロンプト（子タスクの数を表示）
- `--yes` でスキップ
- text/json 出力
- `rm` エイリアス（cli/mod.rs で設定）

## 完了条件

- [x] E2E テストが全て通る
- [x] `--yes` で確認スキップ
- [x] 親タスク削除で子も連動削除
- [x] JSON 出力に `deleted_subtasks` カウントを含む
- [x] 子タスクだけの削除は親に影響しない
- [x] `rm` エイリアスが動作する
