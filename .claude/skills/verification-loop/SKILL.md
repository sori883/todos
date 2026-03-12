# 検証ループスキル

Rust プロジェクト向けの包括的な検証システム。

## 使用タイミング

以下の場合にこのスキルを呼び出す:
- 機能や重要なコード変更の完了後
- PR作成前
- 品質ゲートの通過を確認したい時
- リファクタリング後

## 検証フェーズ

### フェーズ1: ビルド検証
```bash
cargo build 2>&1 | tail -20
```

ビルドが失敗した場合、続行前にSTOPして修正する。

### フェーズ2: Clippy（lint）チェック
```bash
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | head -30
```

全ての clippy 警告を報告する。続行前にクリティカルなものを修正する。

### フェーズ3: フォーマットチェック
```bash
cargo fmt --all -- --check 2>&1 | head -30
```

### フェーズ4: テストスイート
```bash
cargo test 2>&1 | tail -50
```

レポート:
- 総テスト数: X
- パス: X
- フェイル: X

### フェーズ5: セキュリティスキャン
```bash
# シークレットのチェック
grep -rn "sk-\|api_key\|password\s*=" --include="*.rs" src/ 2>/dev/null | head -10

# unsafe ブロックのチェック
grep -rn "unsafe" --include="*.rs" src/ 2>/dev/null | head -10

# 本番コードの .unwrap() チェック（テスト除外）
grep -rn "\.unwrap()" --include="*.rs" src/ 2>/dev/null | head -10
```

### フェーズ6: 差分レビュー
```bash
git diff --stat
git diff HEAD~1 --name-only
```

変更されたファイルごとに以下をレビュー:
- 意図しない変更
- エラーハンドリングの欠落
- 潜在的なエッジケース

## 出力フォーマット

全フェーズ実行後、検証レポートを作成:

```
検証レポート
==================

ビルド:     [PASS/FAIL]
Clippy:     [PASS/FAIL] (X件の警告)
Format:     [PASS/FAIL]
テスト:     [PASS/FAIL] (X/Yパス)
セキュリティ: [PASS/FAIL] (X件の問題)
差分:       [Xファイル変更]

総合:       PR[準備完了/未完了]

修正すべき問題:
1. ...
2. ...
```

## 継続モード

長時間セッションでは、15分ごとまたは大きな変更後に検証を実行:

```markdown
メンタルチェックポイントを設定:
- 各関数の完了後
- モジュールの完成後
- 次のタスクに移る前

実行: /verify
```

## フックとの統合

このスキルはPostToolUseフックを補完するが、より深い検証を提供する。
フックは問題を即座にキャッチし、このスキルは包括的なレビューを提供する。
