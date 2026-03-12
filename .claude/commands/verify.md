# 検証コマンド

現在のコードベースの状態に対して包括的な検証を実行する。

## 手順

以下の正確な順序で検証を実行:

1. **ビルドチェック**
   - `cargo build` を実行
   - 失敗した場合はエラーを報告して停止

2. **Clippy（lint）チェック**
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - 全警告をファイル:行番号で報告

3. **フォーマットチェック**
   - `cargo fmt --all -- --check`
   - フォーマット差分を報告

4. **テストスイート**
   - `cargo test` を実行
   - パス/フェイル数を報告

5. **セキュリティ監査**
   - ソースファイル内のハードコードされたシークレット（API キー、トークン等）を検索
   - src/ 配下の `unsafe` ブロックを検索
   - src/ 配下の `.unwrap()` をリスト化（テストコード除外）

6. **Gitステータス**
   - コミットされていない変更を表示
   - 最後のコミット以降に変更されたファイルを表示

## 出力

簡潔な検証レポートを生成:

```
VERIFICATION: [PASS/FAIL]

Build:    [OK/FAIL]
Clippy:   [OK/X件の警告]
Format:   [OK/差分あり]
Tests:    [X/Yパス]
Security: [OK/X件検出]

PR準備完了: [YES/NO]
```

クリティカルな問題がある場合は、修正提案とともにリスト化する。

## 引数

$ARGUMENTS の指定可能な値:
- `quick` - ビルド + clippy のみ
- `full` - 全チェック（デフォルト）
- `pre-commit` - ビルド + clippy + fmt + テスト
- `pre-pr` - 全チェック + セキュリティスキャン
