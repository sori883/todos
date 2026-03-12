---
name: tdd-workflow
description: Rust プロジェクトでの新機能作成、バグ修正、リファクタリング時にこのスキルを使用する。ユニット、統合、E2Eテストを含む80%以上のカバレッジでテスト駆動開発を強制する。
---

# テスト駆動開発ワークフロー

このスキルは全てのコード開発がTDDの原則に従い、包括的なテストカバレッジを確保する。

## 発動タイミング

- 新しい機能やモジュールの作成時
- バグや問題の修正時
- 既存コードのリファクタリング時
- 新しいサービス関数の追加時
- 新しい CLI コマンドの追加時

## 基本原則

### 1. テストをコードの前に
常にテストを先に書き、テストをパスするコードを実装する。

### 2. カバレッジ要件
- 最低80%カバレッジ（ユニット + 統合 + E2E）
- 全エッジケースをカバー
- エラーシナリオをテスト
- 境界条件を検証

### 3. テストの種類

#### ユニットテスト（`#[cfg(test)] mod tests` - 同一ファイル内）
- 個別の関数とメソッド
- 純粋なロジック（フィルタリング、変換、計算）
- エラーケース（`Result::Err` パス）
- `Option::None` のハンドリング

#### 統合テスト（`tests/` ディレクトリ）
- サービス層の結合テスト
- リポジトリ + サービスの連携
- JSON ファイル I/O
- 設定の読み書き

#### E2Eテスト（`assert_cmd` + `predicates`）
- CLI コマンドの実行と出力検証
- コマンド間の連携フロー
- エラーメッセージの検証

## TDDワークフローステップ

### ステップ1: 型を定義する
```rust
pub struct TaskFilter {
    pub project: Option<String>,
    pub include_done: bool,
}

pub fn filter_tasks(tasks: &[Task], filter: &TaskFilter) -> Vec<&Task> {
    todo!()
}
```

### ステップ2: テストケースを生成
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_by_project_returns_matching_tasks() {
        let tasks = vec![
            Task { project: Some("web".into()), ..Default::default() },
            Task { project: Some("api".into()), ..Default::default() },
        ];
        let filter = TaskFilter { project: Some("web".into()), ..Default::default() };

        let result = filter_tasks(&tasks, &filter);

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn empty_filter_returns_all_active_tasks() {
        let tasks = vec![
            Task { status: Status::Todo, ..Default::default() },
            Task { status: Status::Done, ..Default::default() },
        ];
        let filter = TaskFilter::default();

        let result = filter_tasks(&tasks, &filter);

        assert_eq!(result.len(), 1); // Done は除外
    }

    #[test]
    fn empty_input_returns_empty() {
        let filter = TaskFilter::default();
        let result = filter_tasks(&[], &filter);
        assert!(result.is_empty());
    }
}
```

### ステップ3: テストを実行（失敗するはず）
```bash
cargo test filter
# テストは失敗するはず - todo!() でパニック
```

### ステップ4: コードを実装
テストをパスさせるための最小限のコードを書く。

### ステップ5: テストを再実行
```bash
cargo test filter
# テストがパスするはず
```

### ステップ6: リファクタリング
テストをグリーンに保ちながらコード品質を改善:
- 重複を除去
- 命名を改善
- イテレータチェーンの最適化
- 不要な `.clone()` を除去

### ステップ7: カバレッジを検証
```bash
cargo tarpaulin --skip-clean
# 80%以上のカバレッジを確認
```

## テストパターン

### ユニットテストパターン（Arrange-Act-Assert）
```rust
#[test]
fn sanitize_title_rejects_over_limit() {
    // Arrange
    let settings = Settings { max_title_length: 10, ..Default::default() };
    let input = "a".repeat(11);

    // Act
    let result = sanitize_title(&input, &settings);

    // Assert
    assert!(result.is_err());
}
```

### Result のテスト
```rust
#[test]
fn get_task_returns_error_for_missing_id() {
    let service = create_test_service();
    let result = service.get_task("nonexistent");
    assert!(matches!(result, Err(AppError::TaskNotFound(_))));
}
```

### E2Eテストパターン（assert_cmd）
```rust
#[test]
fn add_and_list_task() {
    let dir = TempDir::new().unwrap();

    // Add
    Command::cargo_bin("todos").unwrap()
        .env("TODOS_DIR", dir.path())
        .args(["add", "--title", "テスト", "--priority", "high"])
        .assert()
        .success();

    // List
    Command::cargo_bin("todos").unwrap()
        .env("TODOS_DIR", dir.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("テスト"));
}
```

## テストファイルの構成

```
src/
├── service/
│   ├── task_service.rs    # 末尾に #[cfg(test)] mod tests
│   └── sanitize.rs        # 末尾に #[cfg(test)] mod tests
├── model/
│   ├── task.rs            # 末尾に #[cfg(test)] mod tests
│   └── filter.rs          # 末尾に #[cfg(test)] mod tests
tests/
├── e2e_01_init.rs         # CLI E2E テスト
├── e2e_03_add_show.rs
├── ...
└── tui_tests.rs           # TUI テスト
```

## 避けるべきよくあるテストの間違い

### ❌ 間違い: 実装の詳細をテスト
```rust
// 内部フィールドの値をテストしない
assert_eq!(service.cache.len(), 1);
```

### ✅ 正しい: 振る舞いをテスト
```rust
// パブリックAPIの結果をテスト
let result = service.list_tasks(&filter)?;
assert_eq!(result.len(), 1);
```

### ❌ 間違い: テスト間の依存
```rust
// テスト順序に依存
static mut SHARED_DATA: Vec<Task> = vec![];
```

### ✅ 正しい: 独立したテスト
```rust
#[test]
fn each_test_has_own_setup() {
    let dir = TempDir::new().unwrap();
    let service = create_test_service_in(&dir);
    // テストロジック
}
```

## カバレッジの検証

### カバレッジレポートの実行
```bash
cargo tarpaulin --out html --output-dir target/tarpaulin
```

### カバレッジ閾値
- **最低80%** 全コードに対して
- **100%必須** 以下のコードに対して:
  - データの永続化（JsonStore の読み書き）
  - 入力サニタイズ（sanitize.rs）
  - エラーハンドリングパス
  - コアビジネスロジック（ステータス遷移、アーカイブ）

## ベストプラクティス

1. **テストを先に書く** - 常にTDD
2. **1テスト1アサーション** - 単一の動作に焦点
3. **説明的なテスト名** - `snake_case` でテスト内容を説明
4. **Arrange-Act-Assert** - 明確なテスト構造
5. **トレイトベースのモック** - 外部依存の抽象化
6. **エッジケースをテスト** - 空文字列、境界値、None
7. **エラーパスをテスト** - `Result::Err` のケース
8. **テストを高速に保つ** - ユニットテストは各50ms未満
9. **テスト後にクリーンアップ** - `TempDir` で自動クリーンアップ
10. **カバレッジレポートをレビュー** - ギャップを特定

---

**注意**: テストはオプションではない。テストは自信を持ったリファクタリング、迅速な開発、プロダクションの信頼性を可能にするセーフティネットである。
