---
name: e2e-test
description: docs/tasks/ の検証項目に基づいて assert_cmd による Rust CLI E2E テストを生成・実行する。TempDir セットアップ、コマンド実行、出力検証、クリーンアップを含む。
---

# E2E テストスキル

`docs/tasks/` の検証セクションを入力とし、`assert_cmd` + `predicates` + `tempfile` による E2E テストを生成・実行する。

## 発動タイミング

- ユーザーが「テストして」「E2E テスト」「検証して」と指示した場合
- 実装タスク完了後に検証セクションの項目をテストする場合
- `docs/tasks/` のファイルをテスト対象として指定された場合

## 前提条件

- `cargo build` が成功する状態
- `Cargo.toml` の `[dev-dependencies]` に `assert_cmd`, `predicates`, `tempfile` がある

## テストファイルの構成

### ファイル配置

```
tests/
├── e2e_01_init.rs
├── e2e_02_config.rs
├── e2e_03_add_show.rs
├── e2e_04_list.rs
├── e2e_05_edit.rs
├── e2e_06_status.rs
├── e2e_07_delete.rs
├── e2e_08_search.rs
├── e2e_09_archive.rs
├── e2e_10_batch.rs
└── tui_tests.rs
```

### 実行方法

```bash
# 全 E2E テスト
cargo test --test 'e2e_*'

# 特定のテストファイル
cargo test --test e2e_03_add_show

# 特定のテスト関数
cargo test --test e2e_03_add_show -- add_with_all_options
```

### テストヘルパーテンプレート

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn setup() -> TempDir {
    let dir = TempDir::new().unwrap();
    // 必要に応じて初期化
    Command::cargo_bin("todos").unwrap()
        .env("TODOS_DIR", dir.path())
        .args(["add", "--title", "seed task", "--priority", "medium"])
        .assert()
        .success();
    dir
}

fn cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("todos").unwrap();
    cmd.env("TODOS_DIR", dir.path());
    cmd
}
```

## テスト作成手順

### 1. 対象タスクの検証項目を読む

```bash
cat "docs/tasks/XX_task_name.md"
```

検証セクション（`## XX.N 検証`）のチェックリストを抽出する。

### 2. テスト対象のコマンドを特定

タスクファイルの各セクションから対象 CLI コマンドを特定する。

### 3. テスト関数を作成

タスクファイルのセクション構成（`## XX.1`, `## XX.2`, ...）に対応させる。

### 4. テストカテゴリ

各テストは以下のいずれかに分類される:

| カテゴリ | テスト方法 | 例 |
|---------|-----------|-----|
| **コマンド実行** | `cmd.assert().success()` で正常終了を検証 | add, edit, delete |
| **出力検証** | `stdout(predicate::str::contains(...))` | list, show, search |
| **JSON出力** | `serde_json::from_slice` でパース→構造検証 | `--format json` |
| **エラーケース** | `cmd.assert().failure()` + stderr 検証 | 不正入力、存在しないID |
| **状態検証** | コマンド実行後に list/show で状態確認 | status, archive |
| **ファイル状態** | TempDir 内のファイルを直接読み取り | tasks.json, archive.json |

### 5. 検証パターン

#### 正常系: コマンド実行と出力

```rust
#[test]
fn add_task_with_all_options() {
    let dir = TempDir::new().unwrap();
    cmd(&dir)
        .args(["add", "--title", "テスト", "--priority", "high", "--content", "詳細"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));
}
```

#### JSON出力の構造検証

```rust
#[test]
fn list_returns_valid_json() {
    let dir = setup();
    let output = cmd(&dir)
        .args(["list", "--format", "json"])
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["success"].as_bool().unwrap());
    assert!(!json["data"].as_array().unwrap().is_empty());
}
```

#### エラーケース

```rust
#[test]
fn show_nonexistent_task_fails() {
    let dir = TempDir::new().unwrap();
    cmd(&dir)
        .args(["show", "00000000"])
        .assert()
        .failure();
}
```

#### 状態遷移の検証

```rust
#[test]
fn done_archives_task() {
    let dir = setup();
    let prefix = get_first_task_prefix(&dir);

    // Done に変更 → アーカイブ
    cmd(&dir)
        .args(["status", &prefix, "done"])
        .assert()
        .success();

    // 通常の list には表示されない
    cmd(&dir)
        .args(["list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("seed task").not());

    // archive コマンドで表示される
    cmd(&dir)
        .args(["archive", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("seed task"));
}
```

## クリーンアップ

`TempDir` は drop 時に自動削除されるため、明示的なクリーンアップは不要。

## 重要な注意点

- **テスト分離**: 各テストは独自の `TempDir` を使い、テスト間の干渉を防ぐ
- **ID の取得**: JSON 出力からタスク ID を取得し、prefix（先頭8文字）を使う
- **出力の部分一致**: `predicate::str::contains()` で部分一致テスト。完全一致は脆い
- **exit code**: 成功は `.success()`、失敗は `.failure()` で検証
- **環境変数**: `TODOS_DIR` でデータディレクトリを TempDir に向ける
