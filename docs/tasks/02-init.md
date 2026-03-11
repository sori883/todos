# 02: init コマンド

## 概要

`todos init` で `.todos/` ディレクトリと初期ファイルを作成する。`.todos/` の上方探索ロジックも実装する。

## 依存タスク

01-project-setup

## 先に書く E2E テスト

```rust
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
    assert_eq!(data["version"], 1);
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
    todos_cmd(dir.path()).args(["init", "--force"]).assert().success();
}

#[test]
fn init_json_output() {
    let dir = TempDir::new().unwrap();
    let json = todos_json(dir.path(), &["init"]);
    assert_eq!(json["success"], true);
}
```

## 実装内容

### 1. config/paths.rs

- `fn find_data_dir(start: &Path) -> PathBuf` -- カレントから上方に `.todos/` を探索。ホームディレクトリが上限。見つからなければ `~/.config/todos/`
- `fn resolve_data_dir(global: bool, explicit: Option<&Path>) -> PathBuf` -- グローバルオプション処理

### 2. store/schema.rs

- `TaskData` 構造体（`version: u32`, `tasks: Vec<Task>`）
- `fn create_empty() -> TaskData` -- version=1, tasks=[]
- `fn validate_version(data: &Value) -> Result<()>`

### 3. cli/init.rs

- `.todos/` ディレクトリ作成
- `tasks.json` に空データを書き込み
- `--force` オプション
- text/json 出力対応

### 4. cli/mod.rs

- clap の `Init` サブコマンド定義
- グローバルオプション（`--format`, `--yes`, `--global`, `--data-dir`）定義

### 5. cli/output.rs

- `CliResponse<T>` 構造体
- text/json 出力切り替え
- エラーの stdout(json)/stderr(text) 振り分け

### 6. error.rs

- `AppError` の基本バリアント（`DataFile`, `Config`）

## 完了条件

- [ ] E2E テストが全て通る
- [ ] `todos init` で `.todos/tasks.json` が作成される
- [ ] `--force` なしで既存上書き拒否
- [ ] `--format json` で JSON レスポンス
- [ ] `--data-dir` でディレクトリ指定可能
