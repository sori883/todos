---
description: テスト駆動開発ワークフローを強制する。構造体/トレイト定義、テストを先に生成、次に最小限のコードで実装。80%以上のカバレッジを確保。
---

# TDDコマンド

このコマンドは**tdd-guide**エージェントを呼び出し、テスト駆動開発の方法論を強制する。

## このコマンドの機能

1. **構造体/トレイトの定義** - 型とインターフェースを最初に定義
2. **テストを先に生成** - 失敗するテストを書く（RED）
3. **最小限のコードで実装** - パスするのに十分なコードのみ書く（GREEN）
4. **リファクタリング** - テストをグリーンに保ちながらコードを改善（REFACTOR）
5. **カバレッジを検証** - 80%以上のテストカバレッジを確保

## 使用タイミング

以下の場合に `/tdd` を使用:
- 新機能の実装
- 新しい関数/モジュールの追加
- バグ修正（まずバグを再現するテストを書く）
- 既存コードのリファクタリング
- クリティカルなビジネスロジックの構築

## 動作の仕組み

tdd-guideエージェントが以下を実行:

1. **構造体/トレイトを定義** 入出力用
2. **失敗するテストを書く** （コードがまだ存在しないため）
3. **テストを実行** し正しい理由で失敗することを確認
4. **最小限の実装を書く** テストをパスさせるため
5. **テストを実行** しパスすることを確認
6. **コードをリファクタリング** テストをグリーンに保ちながら
7. **カバレッジを確認** 80%未満ならテストを追加

## TDDサイクル

```
RED → GREEN → REFACTOR → REPEAT

RED:      失敗するテストを書く
GREEN:    パスするための最小限のコードを書く
REFACTOR: テストをパスさせたままコードを改善
REPEAT:   次の機能/シナリオへ
```

## 使用例

```
ユーザー: /tdd タスクのフィルタリング関数が必要

エージェント（tdd-guide）:
# TDDセッション: タスクフィルタリング

## ステップ1: 型の定義（SCAFFOLD）

```rust
// src/model/filter.rs
#[derive(Debug, Default)]
pub struct TaskFilter {
    pub project: Option<String>,
    pub label: Option<String>,
    pub priority: Option<Priority>,
    pub include_done: bool,
}

pub fn filter_tasks(tasks: &[Task], filter: &TaskFilter) -> Vec<&Task> {
    todo!()
}
```

## ステップ2: 失敗するテストを書く（RED）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tasks() -> Vec<Task> {
        vec![
            Task { title: "Task A".into(), project: Some("web".into()), ..Default::default() },
            Task { title: "Task B".into(), project: Some("api".into()), ..Default::default() },
            Task { title: "Task C".into(), project: Some("web".into()), status: Status::Done, ..Default::default() },
        ]
    }

    #[test]
    fn filter_by_project() {
        let tasks = sample_tasks();
        let filter = TaskFilter { project: Some("web".into()), ..Default::default() };
        let result = filter_tasks(&tasks, &filter);
        assert_eq!(result.len(), 1); // Done は除外
        assert_eq!(result[0].title, "Task A");
    }

    #[test]
    fn filter_includes_done_when_flag_set() {
        let tasks = sample_tasks();
        let filter = TaskFilter {
            project: Some("web".into()),
            include_done: true,
            ..Default::default()
        };
        let result = filter_tasks(&tasks, &filter);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn empty_filter_returns_active_tasks() {
        let tasks = sample_tasks();
        let filter = TaskFilter::default();
        let result = filter_tasks(&tasks, &filter);
        assert_eq!(result.len(), 2); // Done 除外
    }
}
```

## ステップ3: テスト実行 - 失敗を確認

```bash
cargo test filter

running 3 tests
test model::filter::tests::filter_by_project ... FAILED
test model::filter::tests::filter_includes_done ... FAILED
test model::filter::tests::empty_filter_returns_active ... FAILED

3 tests failed
```

✅ テストが期待通りに失敗（`todo!()` パニック）。実装準備完了。

## ステップ4: 最小限のコードで実装（GREEN）

```rust
pub fn filter_tasks<'a>(tasks: &'a [Task], filter: &TaskFilter) -> Vec<&'a Task> {
    tasks.iter()
        .filter(|t| {
            if !filter.include_done && t.status == Status::Done {
                return false;
            }
            if let Some(ref proj) = filter.project {
                if t.project.as_deref() != Some(proj.as_str()) {
                    return false;
                }
            }
            true
        })
        .collect()
}
```

## ステップ5: テスト実行 - パスを確認

```bash
cargo test filter

running 3 tests
test model::filter::tests::filter_by_project ... ok
test model::filter::tests::filter_includes_done ... ok
test model::filter::tests::empty_filter_returns_active ... ok

3 tests passed
```

✅ 全テストパス！

## ステップ6: リファクタリング（IMPROVE）

テストがグリーンのまま、可読性を改善。

## ステップ7: カバレッジの確認

```bash
cargo tarpaulin --skip-clean -- filter
```
```

## TDDベストプラクティス

**推奨:**
- ✅ 実装の前にまずテストを書く
- ✅ 実装前にテストを実行し失敗を確認
- ✅ テストをパスさせるための最小限のコードを書く
- ✅ テストがグリーンになってからリファクタリング
- ✅ エッジケースとエラーシナリオを追加
- ✅ 80%以上のカバレッジを目標（クリティカルなコードは100%）

**非推奨:**
- ❌ テスト前に実装を書く
- ❌ 各変更後のテスト実行をスキップ
- ❌ 一度に大量のコードを書く
- ❌ 失敗するテストを無視
- ❌ 実装の詳細をテスト（振る舞いをテストすること）

## 含めるべきテストタイプ

**ユニットテスト**（`#[cfg(test)] mod tests` - 同一ファイル内）:
- 個別の関数とユーティリティ
- 構造体のメソッド
- エラーケース（`Result::Err` パス）
- 境界値

**統合テスト**（`tests/` ディレクトリ）:
- サービス層の連携テスト
- リポジトリ + サービスの結合
- JSON ファイル I/O

**E2Eテスト**（`assert_cmd` + `predicates`）:
- CLI コマンドの実行と出力検証
- JSON 出力のパース
- エラーメッセージの検証

## カバレッジ要件

- **最低80%** 全コードに対して
- **100%必須** 以下のコードに対して:
  - データの永続化ロジック
  - 入力バリデーション/サニタイズ
  - エラーハンドリングパス
  - コアビジネスロジック

## 重要な注意事項

**MANDATORY**: テストは実装の前に書くこと。TDDサイクルは:

1. **RED** - 失敗するテストを書く
2. **GREEN** - パスするよう実装
3. **REFACTOR** - コードを改善

REDフェーズを絶対にスキップしないこと。テスト前にコードを書かないこと。

## 他のコマンドとの連携

- `/plan` で最初に何を構築するか理解
- `/tdd` でテスト付きで実装
- `/build-fix` でビルドエラーが発生した場合の対応
- `/code-review` で実装のレビュー
- `/test-coverage` でカバレッジの検証
