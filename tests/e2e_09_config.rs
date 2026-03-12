mod helpers;

use helpers::{setup, todos_cmd, todos_json};

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
        .assert()
        .success();
    let json = todos_json(dir.path(), &["config", "--show"]);
    assert_eq!(json["data"]["keybindings"]["mode"], "vi");
}

#[test]
fn config_icons_nerd() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["config", "--icons", "nerd"])
        .assert()
        .success();
    let json = todos_json(dir.path(), &["config", "--show"]);
    assert_eq!(json["data"]["icons"]["style"], "nerd");
}

#[test]
fn config_reset_local() {
    let dir = setup();
    // ローカル設定を作成
    todos_cmd(dir.path())
        .args(["config", "--mode", "vi"])
        .assert()
        .success();
    assert!(dir.path().join(".todos/settings.json").exists());
    // リセット
    todos_cmd(dir.path())
        .args(["config", "--reset", "--yes"])
        .assert()
        .success();
    assert!(!dir.path().join(".todos/settings.json").exists());
}

#[test]
fn config_reset_no_local() {
    let dir = setup();
    // ローカル設定がない状態でリセット
    todos_cmd(dir.path())
        .args(["config", "--reset"])
        .assert()
        .success(); // エラーではなくメッセージ表示
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
        .assert()
        .success();
}

#[test]
fn extra_labels_invalid_still_fails() {
    let dir = setup();
    let settings = r#"{"extra_labels": ["perf"]}"#;
    std::fs::write(dir.path().join(".todos/settings.json"), settings).unwrap();
    // extra にもない label はエラー
    todos_cmd(dir.path())
        .args(["add", "テスト", "-l", "unknown"])
        .assert()
        .failure();
}
