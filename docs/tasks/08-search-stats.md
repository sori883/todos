# 08: search + stats コマンド

## 概要

テキスト検索と統計情報表示。

## 依存タスク

04-list

## 先に書く E2E テスト

### search

```rust
#[test]
fn search_by_title() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "API実装"]).assert().success();
    todos_cmd(dir.path()).args(["add", "テスト追加"]).assert().success();
    let json = todos_json(dir.path(), &["search", "API"]);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["tasks"][0]["title"], "API実装");
}

#[test]
fn search_by_description() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "タスク1", "-d", "認証機能の実装"]).assert().success();
    todos_cmd(dir.path()).args(["add", "タスク2", "-d", "テスト追加"]).assert().success();
    let json = todos_json(dir.path(), &["search", "認証"]);
    assert_eq!(json["data"]["count"], 1);
}

#[test]
fn search_case_insensitive() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "API endpoint"]).assert().success();
    let json = todos_json(dir.path(), &["search", "api"]);
    assert_eq!(json["data"]["count"], 1);
}

#[test]
fn search_with_filter() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "API実装", "-l", "feature"]).assert().success();
    todos_cmd(dir.path()).args(["add", "APIバグ修正", "-l", "bug"]).assert().success();
    let json = todos_json(dir.path(), &["search", "API", "-l", "bug"]);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["tasks"][0]["label"], "bug");
}

#[test]
fn search_no_results() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "テスト"]).assert().success();
    let json = todos_json(dir.path(), &["search", "存在しない"]);
    assert_eq!(json["data"]["count"], 0);
}

#[test]
fn search_is_alias_for_list_query() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "検索対象"]).assert().success();
    let search = todos_json(dir.path(), &["search", "検索"]);
    let list = todos_json(dir.path(), &["list", "-q", "検索"]);
    assert_eq!(search["data"]["count"], list["data"]["count"]);
}
```

### stats

```rust
#[test]
fn stats_empty() {
    let dir = setup();
    let json = todos_json(dir.path(), &["stats"]);
    assert_eq!(json["data"]["total"], 0);
}

#[test]
fn stats_counts() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "A", "-p", "high", "-l", "bug", "-P", "svc-a"]).assert().success();
    todos_cmd(dir.path()).args(["add", "B", "-p", "low", "-l", "feature", "-P", "svc-a"]).assert().success();
    todos_cmd(dir.path()).args(["add", "C", "-p", "high", "-l", "bug", "-P", "svc-b", "-c", "ai"]).assert().success();
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
    todos_cmd(dir.path()).args(["add", "A", "-P", "svc-a"]).assert().success();
    todos_cmd(dir.path()).args(["add", "B", "-P", "svc-b"]).assert().success();
    let json = todos_json(dir.path(), &["stats", "-P", "svc-a"]);
    assert_eq!(json["data"]["total"], 1);
}
```

## 実装内容

### 1. service/task_service.rs

- `search_tasks()` -- title/description 部分一致（大文字小文字無視）+ フィルタ
- `stats()` -- フィルタ適用後の集計

### 2. cli/list.rs（search エイリアス）

- `search` サブコマンドを `list --query` にマッピング

### 3. cli/stats.rs

- フィルタオプション定義
- text/json 出力

## 完了条件

- [x] E2E テストが全て通る
- [x] title/description の部分一致検索
- [x] 大文字小文字を区別しない検索
- [x] search + フィルタの組み合わせ
- [x] stats の全カテゴリ集計（status, priority, label, project, creator）
- [x] stats にフィルタ適用可能
