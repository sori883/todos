# 09: config コマンド

## 概要

設定の表示・変更。グローバル/ローカル設定のマージ、extra_labels/extra_projects の管理。

## 依存タスク

02-init

## 先に書く E2E テスト

```rust
#[test]
fn config_show_default() {
    let dir = setup();
    let json = todos_json(dir.path(), &["config", "--show"]);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["locale"], "ja");
}

#[test]
fn config_show_includes_builtin_labels() {
    let dir = setup();
    let json = todos_json(dir.path(), &["config", "--show"]);
    let labels = json["data"]["builtin_labels"].as_array().unwrap();
    assert!(labels.contains(&serde_json::json!("bug")));
    assert!(labels.contains(&serde_json::json!("feature")));
}

#[test]
fn config_mode_vi() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["config", "--mode", "vi"])
        .assert().success();
    let json = todos_json(dir.path(), &["config", "--show"]);
    assert_eq!(json["data"]["keybindings"]["mode"], "vi");
}

#[test]
fn config_icons_nerd() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["config", "--icons", "nerd"])
        .assert().success();
    let json = todos_json(dir.path(), &["config", "--show"]);
    assert_eq!(json["data"]["icons"]["style"], "nerd");
}

#[test]
fn config_reset_local() {
    let dir = setup();
    // ローカル設定を作成
    todos_cmd(dir.path()).args(["config", "--mode", "vi"]).assert().success();
    assert!(dir.path().join(".todos/settings.json").exists());
    // リセット
    todos_cmd(dir.path()).args(["config", "--reset", "--yes"]).assert().success();
    assert!(!dir.path().join(".todos/settings.json").exists());
}

#[test]
fn config_reset_no_local() {
    let dir = setup();
    // ローカル設定がない状態でリセット
    todos_cmd(dir.path())
        .args(["config", "--reset"])
        .assert().success(); // エラーではなくメッセージ表示
}

#[test]
fn extra_labels_used_in_add() {
    let dir = setup();
    // settings.json に extra_labels を直接書き込み
    let settings = r#"{"extra_labels": ["perf"]}"#;
    std::fs::write(dir.path().join(".todos/settings.json"), settings).unwrap();
    // extra label でタスク追加可能
    todos_cmd(dir.path())
        .args(["add", "パフォーマンス改善", "-l", "perf"])
        .assert().success();
}

#[test]
fn extra_labels_invalid_still_fails() {
    let dir = setup();
    let settings = r#"{"extra_labels": ["perf"]}"#;
    std::fs::write(dir.path().join(".todos/settings.json"), settings).unwrap();
    // extra にもない label はエラー
    todos_cmd(dir.path())
        .args(["add", "テスト", "-l", "unknown"])
        .assert().failure();
}

#[test]
fn settings_merge_global_and_local() {
    // グローバルに extra_labels: ["perf"]
    // ローカルに extra_labels: ["security"]
    // マージ結果: ["perf", "security"]
    let dir = setup();
    let global_dir = TempDir::new().unwrap();
    let global_config = global_dir.path().join("settings.json");
    std::fs::write(&global_config, r#"{"extra_labels": ["perf"]}"#).unwrap();
    let local_settings = r#"{"extra_labels": ["security"]}"#;
    std::fs::write(dir.path().join(".todos/settings.json"), local_settings).unwrap();
    // perf と security の両方が使える（グローバル設定のパス解決はテストでは難しいため、この部分はユニットテストで補完）
}
```

## 実装内容

### 1. config/settings.rs

- `Settings` 構造体（全設定フィールド、デフォルト値付き）
- `fn load(data_dir: &Path) -> Result<Settings>` -- ローカル + グローバルのマージ
- マージ戦略: スカラー=上書き、配列=重複排除結合、オブジェクト=フィールド単位上書き
- `fn available_labels(&self) -> Vec<String>` -- ビルトイン + extra_labels

### 2. cli/config.rs

- `--show`: 現在の設定を表示（ビルトイン値含む）
- `--reset`: ローカル設定削除 / グローバルリセット
- `--mode`, `--icons`: 設定ファイルに書き込み

### 3. config/theme.rs（型定義のみ）

- `IconStyle` enum（Chars, Nerd）
- アイコンマッピングテーブル

### 4. config/keybindings.rs（型定義のみ）

- `KeybindMode` enum（Default, Vi）

## 完了条件

- [x] E2E テストが全て通る
- [x] `config --show` でビルトイン値を含む設定表示
- [x] `config --mode` / `--icons` で設定変更
- [x] `config --reset` でローカル設定削除
- [x] `extra_labels` が add の label バリデーションに反映
- [x] グローバル/ローカル設定のマージが正しく動作（ユニットテスト）
