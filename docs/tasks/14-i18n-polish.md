# 14: i18n・仕上げ

## 概要

多言語対応（enum + match 方式）、エッジケーステスト追加、エラーメッセージ改善、パフォーマンス検証。

## 依存タスク

13-tui-form-actions

## 先に書く E2E テスト

```rust
#[test]
fn locale_ja_messages() {
    let dir = setup();
    // デフォルト locale=ja
    let json = todos_json(dir.path(), &["add", "テスト"]);
    assert_eq!(json["success"], true);
    // メッセージが日本語
    assert!(json["message"].as_str().unwrap().contains("作成"));
}

#[test]
fn locale_en_messages() {
    let dir = setup();
    // locale を en に変更
    let settings = r#"{"locale": "en"}"#;
    std::fs::write(dir.path().join(".todos/settings.json"), settings).unwrap();
    let json = todos_json(dir.path(), &["add", "test task"]);
    assert!(json["message"].as_str().unwrap().contains("created"));
}

#[test]
fn invalid_locale_falls_back_to_en() {
    let dir = setup();
    let settings = r#"{"locale": "xx"}"#;
    std::fs::write(dir.path().join(".todos/settings.json"), settings).unwrap();
    let json = todos_json(dir.path(), &["add", "test"]);
    // en にフォールバック
    assert!(json["message"].as_str().unwrap().contains("created"));
}

#[test]
fn date_format_respects_settings() {
    let dir = setup();
    let settings = r#"{"date_formats": {"display": "%d/%m/%Y"}}"#;
    std::fs::write(dir.path().join(".todos/settings.json"), settings).unwrap();
    todos_cmd(dir.path()).args(["add", "テスト"]).assert().success();
    let json = todos_json(dir.path(), &["show", "--format", "json"]);
    // 表示用日付がカスタムフォーマット
    let created = json["data"]["task"]["created_at_display"].as_str().unwrap();
    assert!(created.contains("/"));
}

#[test]
fn large_dataset_performance() {
    let dir = setup();
    // 500 タスクを batch で投入
    let actions: Vec<String> = (0..500)
        .map(|i| format!(r#"{{"action": "add", "title": "タスク{}"}}"#, i))
        .collect();
    let input = format!("[{}]", actions.join(","));
    todos_json_stdin(dir.path(), &["batch"], &input);
    // list が 2 秒以内に返る
    let start = std::time::Instant::now();
    todos_cmd(dir.path()).args(["list"]).assert().success();
    assert!(start.elapsed().as_secs() < 2);
}

#[test]
fn error_message_task_not_found() {
    let dir = setup();
    let output = todos_cmd(dir.path())
        .args(["show", "0000"])
        .assert().failure()
        .get_output().stdout.clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().len() > 0);
}

#[test]
fn error_message_ambiguous_id() {
    let dir = setup();
    // 同じプレフィクスを持つタスクが複数ある状況は作りにくいため、
    // 短すぎる ID でエラーを確認
    let output = todos_cmd(dir.path())
        .args(["show", "ab"])
        .assert().failure()
        .get_output().stdout.clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().len() > 0);
}

#[test]
fn help_text_shows_all_commands() {
    todos_cmd_no_init()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("add"))
        .stdout(predicates::str::contains("list"))
        .stdout(predicates::str::contains("show"))
        .stdout(predicates::str::contains("edit"))
        .stdout(predicates::str::contains("status"))
        .stdout(predicates::str::contains("delete"))
        .stdout(predicates::str::contains("search"))
        .stdout(predicates::str::contains("stats"))
        .stdout(predicates::str::contains("config"))
        .stdout(predicates::str::contains("batch"));
}
```

## 実装内容

### 1. i18n/mod.rs

- `Message` enum（全 UI テキストを列挙）
- `get_message(msg, locale) -> &'static str`
- サポートロケール: `ja`（デフォルト）, `en`
- 不明なロケールは `en` にフォールバック + 警告

### 2. i18n/ja.rs

- 日本語メッセージ定義
  - CLI 出力メッセージ（タスク作成、編集、削除、ステータス変更等）
  - エラーメッセージ（タスク未発見、ID 曖昧、バリデーション等）
  - TUI ラベル（画面タイトル、フォームラベル、ヘルプバー等）

### 3. i18n/en.rs

- 英語メッセージ定義（同じキーセット）

### 4. 既存コードの i18n 対応

- CLI 出力のハードコード文字列を `get_message()` に置換
- TUI のラベル・ヘルプテキストを `get_message()` に置換
- エラーメッセージを `get_message()` に置換

### 5. エラーメッセージ改善

- 全エラーに具体的なコンテキストを付与
  - 「タスクが見つかりません」→「ID 'abcd1234' に一致するタスクが見つかりません」
  - 「ID が曖昧です」→「'ab' に一致するタスクが 3 件あります。4 文字以上で指定してください」
- JSON エラー出力の一貫性確認

### 6. パフォーマンス検証

- 500 タスクでの list / search / stats のレスポンス確認
- 不要な再読み込みの排除

### 7. エッジケーステスト追加

- 空文字タイトル
- 非常に長いタイトル（1000 文字）
- Unicode 特殊文字（絵文字、結合文字、RTL）
- 同時書き込み（ファイルロック動作確認）
- 壊れた JSON ファイルからのリカバリ

## 完了条件

- [ ] E2E テストが全て通る
- [ ] `locale: "ja"` で日本語メッセージ表示
- [ ] `locale: "en"` で英語メッセージ表示
- [ ] 不明ロケールで `en` フォールバック
- [ ] `date_formats` 設定が表示に反映
- [ ] エラーメッセージに具体的なコンテキスト付き
- [ ] 500 タスクで list が 2 秒以内
- [ ] ヘルプテキストに全コマンド表示
- [ ] エッジケーステストが通る
