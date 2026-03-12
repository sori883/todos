---
description: assert_cmd + predicates を使用して Rust CLI の E2E テストを生成・実行する。コマンド実行、出力検証、ファイル状態確認を含む。
---

# E2Eコマンド

このコマンドは Rust CLI プロジェクト向けの E2E テストを生成・実行する。

## このコマンドの機能

1. **テストシナリオの生成** - CLI コマンドの E2E テストを作成
2. **E2Eテストの実行** - `cargo test --test e2e_*` でテストを実行
3. **出力検証** - CLI の stdout/stderr/終了コードを検証
4. **JSON出力のパース** - `--format json` の出力を構造的に検証

## 使用タイミング

以下の場合に `/e2e` を使用:
- CLI コマンドの完全なワークフローテスト
- コマンド間の連携テスト（add → list → show → edit → delete）
- エラーケースの検証（不正な入力、存在しないリソース）
- JSON 出力フォーマットの検証

## 動作の仕組み

1. **ユーザーフローを分析** しテストシナリオを特定
2. **テストコードを生成** `assert_cmd` + `predicates` + `tempfile` を使用
3. **テストを実行** `cargo test --test <test_file>`
4. **結果を報告** パス/フェイル数とエラー詳細

## 使用例

```
ユーザー: /e2e タスクの追加から削除までのフローをテスト

# E2Eテスト生成: タスクライフサイクル

## テストシナリオ

**ユーザーフロー:** タスク追加 → 一覧確認 → 詳細表示 → 編集 → ステータス変更 → 削除

## 生成されたテストコード

```rust
// tests/e2e_lifecycle.rs
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("todos").unwrap();
    cmd.env("TODOS_DIR", dir.path());
    cmd
}

#[test]
fn task_full_lifecycle() {
    let dir = TempDir::new().unwrap();

    // 1. タスクを追加
    cmd(&dir)
        .args(["add", "--title", "E2Eテスト", "--priority", "high"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));

    // 2. 一覧に表示される
    cmd(&dir)
        .args(["list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("E2Eテスト"));

    // 3. 詳細表示
    let output = cmd(&dir)
        .args(["list", "--format", "json"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let id = json["data"][0]["id"].as_str().unwrap();
    let prefix = &id[..8];

    cmd(&dir)
        .args(["show", prefix])
        .assert()
        .success()
        .stdout(predicate::str::contains("E2Eテスト"));

    // 4. 編集
    cmd(&dir)
        .args(["edit", prefix, "--title", "更新済みタスク"])
        .assert()
        .success();

    // 5. ステータス変更
    cmd(&dir)
        .args(["status", prefix, "done"])
        .assert()
        .success();

    // 6. 削除
    cmd(&dir)
        .args(["delete", prefix, "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted"));
}

#[test]
fn invalid_command_shows_error() {
    let dir = TempDir::new().unwrap();

    cmd(&dir)
        .args(["show", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("error")));
}
```

## テスト実行

```bash
cargo test --test e2e_lifecycle

running 2 tests
test task_full_lifecycle ... ok
test invalid_command_shows_error ... ok

2 tests passed
```
```

## テスト構成

```
tests/
├── e2e_01_init.rs        # 初期化テスト
├── e2e_02_config.rs      # 設定テスト
├── e2e_03_add_show.rs    # 追加・表示テスト
├── e2e_04_list.rs        # 一覧テスト
├── e2e_05_edit.rs        # 編集テスト
├── e2e_06_status.rs      # ステータス変更テスト
├── e2e_07_delete.rs      # 削除テスト
├── e2e_08_search.rs      # 検索テスト
├── e2e_09_archive.rs     # アーカイブテスト
├── e2e_10_batch.rs       # バッチテスト
└── tui_tests.rs          # TUI テスト
```

## テストパターン

### 基本パターン: コマンド実行と出力検証
```rust
cmd(&dir)
    .args(["command", "arg1", "--flag", "value"])
    .assert()
    .success()
    .stdout(predicate::str::contains("期待される出力"));
```

### JSON出力の検証
```rust
let output = cmd(&dir).args(["list", "--format", "json"]).output().unwrap();
let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
assert_eq!(json["data"].as_array().unwrap().len(), 3);
```

### エラーケースの検証
```rust
cmd(&dir)
    .args(["add"]) // 必須引数なし
    .assert()
    .failure();
```

## ベストプラクティス

**推奨:**
- ✅ `TempDir` で各テストを分離
- ✅ `--format json` で構造的な出力検証
- ✅ 正常系とエラー系の両方をテスト
- ✅ コマンド間の連携をテスト
- ✅ テスト後の自動クリーンアップ（`TempDir` の drop）

**非推奨:**
- ❌ テスト間でデータを共有
- ❌ 出力の完全一致テスト（部分一致を使用）
- ❌ テスト順序に依存
- ❌ ハードコードされた UUID やタイムスタンプの検証

## CI/CD統合

```yaml
- name: E2Eテストを実行
  run: cargo test --test 'e2e_*'
```

## 他のコマンドとの連携

- `/plan` でテストすべきクリティカルなフローを特定
- `/tdd` でユニットテスト（より高速、より細粒度）
- `/e2e` で CLI コマンドの統合テスト
- `/code-review` でテスト品質を検証
