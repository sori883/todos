# 実装タスク概要

## 開発方針: E2E テストファースト

各タスクは以下の順序で進める:

1. **E2E テストを書く** -- 期待する CLI の入出力を `assert_cmd` で定義
2. **実装する** -- テストが通るように必要なコードを実装
3. **検証する** -- `cargo test` で全テストが通ることを確認

### テスト基盤

| 用途 | クレート |
|---|---|
| CLI E2E テスト | `assert_cmd` (dev-dependency) |
| テスト用一時ディレクトリ | `tempfile` (dev-dependency) |
| JSON 出力検証 | `serde_json` (dev-dependency) |
| 出力文字列マッチ | `predicates` (dev-dependency) |

### E2E テストパターン

```rust
use assert_cmd::Command;
use tempfile::TempDir;

fn todos_cmd(data_dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("todos").unwrap();
    cmd.arg("--data-dir").arg(data_dir);
    cmd
}

#[test]
fn test_example() {
    let dir = TempDir::new().unwrap();
    // 1. init
    todos_cmd(dir.path()).args(["init"]).assert().success();
    // 2. 操作
    todos_cmd(dir.path()).args(["add", "テスト"]).assert().success();
    // 3. 検証
    let output = todos_cmd(dir.path())
        .args(["list", "--format", "json"])
        .assert().success()
        .get_output().stdout.clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["data"]["count"], 1);
}
```

### TUI テスト方針

TUI は `ratatui` の `TestBackend` でレンダリング結果を検証する統合テストを書く。ビジネスロジックは CLI E2E テストでカバー済みのため、TUI テストは表示とキーバインドに集中する。

---

## タスク一覧

| # | タスク | 依存 | 状態 |
|---|---|---|---|
| 01 | [プロジェクトセットアップ](./01-project-setup.md) | - | [x] |
| 02 | [init コマンド](./02-init.md) | 01 | [x] |
| 03 | [add + show コマンド](./03-add-show.md) | 02 | [x] |
| 04 | [list コマンド](./04-list.md) | 03 | [x] |
| 05 | [edit コマンド](./05-edit.md) | 03 | [x] |
| 06 | [status コマンド](./06-status.md) | 03 | [x] |
| 07 | [delete コマンド](./07-delete.md) | 03 | [x] |
| 08 | [search + stats コマンド](./08-search-stats.md) | 04 | [x] |
| 09 | [config コマンド](./09-config.md) | 02 | [x] |
| 10 | [batch コマンド](./10-batch.md) | 06, 05, 07 | [x] |
| 11 | [TUI 基盤](./11-tui-foundation.md) | 03 | [x] |
| 12 | [TUI 一覧・詳細画面](./12-tui-list-detail.md) | 11 | [x] |
| 13 | [TUI フォーム・操作](./13-tui-form-actions.md) | 12 | [x] |
| 14 | [i18n・仕上げ](./14-i18n-polish.md) | 13 | [x] |
| -- | ~~繰り返しタスク (recurrence)~~ | - | 削除 |
| 15 | アーカイブ機能（Done/Cancelled 時に archive.json へ移動・復元） | 06 | [x] |

### 依存関係図

```
01 → 02 → 03 → 04 → 08
               ├→ 05 ─┐
               ├→ 06 ─┼→ 10
               ├→ 07 ─┘
               └→ 11 → 12 → 13 → 14
         02 → 09
```
