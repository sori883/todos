mod helpers;

use helpers::*;

// === i18n locale tests ===

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

// === Performance test ===

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

// === Error message tests ===

#[test]
fn error_message_task_not_found() {
    let dir = setup();
    let output = todos_cmd(dir.path())
        .args(["show", "0000", "--format", "json"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().len() > 0);
}

#[test]
fn error_message_ambiguous_id() {
    let dir = setup();
    // 短すぎる ID でエラーを確認
    let output = todos_cmd(dir.path())
        .args(["show", "ab", "--format", "json"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().len() > 0);
}

// === Help test ===

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

// === Edge case tests ===

#[test]
fn add_unicode_emoji_title() {
    let dir = setup();
    let json = todos_json(dir.path(), &["add", "🎉 パーティー準備"]);
    assert_eq!(json["data"]["task"]["title"], "🎉 パーティー準備");
}

#[test]
fn add_long_title() {
    let dir = setup();
    let long_title = "a".repeat(1000);
    let json = todos_json(dir.path(), &["add", &long_title]);
    assert_eq!(json["data"]["task"]["title"].as_str().unwrap().len(), 1000);
}
