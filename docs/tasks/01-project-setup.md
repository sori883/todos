# 01: プロジェクトセットアップ

## 概要

Cargo プロジェクトの初期化、依存クレートの追加、ディレクトリ構造のスキャフォールド、E2E テスト基盤の構築。

## 依存タスク

なし

## 実装内容

### 1. Cargo.toml

```toml
[package]
name = "todos"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
ratatui = "0.29"
crossterm = "0.29"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
anyhow = "1"
thiserror = "2"
dirs = "5"
fs2 = "0.4"
itertools = "0.13"
unicode-width = "0.2"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

### 2. ディレクトリ構造

```
src/
├── main.rs          # エントリポイント（空の main + clap 最小定義）
├── lib.rs           # pub mod 宣言
├── error.rs         # AppError（空の enum）
├── model/
│   └── mod.rs
├── service/
│   └── mod.rs
├── store/
│   └── mod.rs
├── cli/
│   └── mod.rs
├── tui/
│   └── mod.rs
├── config/
│   └── mod.rs
└── i18n/
    └── mod.rs
tests/
└── helpers/
    └── mod.rs       # E2E テスト用ヘルパー（todos_cmd 関数）
```

### 3. E2E テストヘルパー

`tests/helpers/mod.rs` にテスト用共通関数を定義:
- `todos_cmd(data_dir)` -- `--data-dir` 付きの Command を返す
- `todos_json(data_dir, args)` -- `--format json` 付きで実行し JSON を返す

### 4. 最小の main.rs

`clap` で空のコマンド定義を作成。引数なしで `println!("TUI is not implemented yet")` を表示、サブコマンドがあれば `println!("Not implemented")` を表示して終了。

## E2E テスト

```rust
#[test]
fn binary_exists_and_runs() {
    Command::cargo_bin("todos").unwrap()
        .arg("--help")
        .assert()
        .success();
}
```

## 完了条件

- [x] `cargo build` が成功する
- [x] `cargo test` が成功する（binary_exists テスト通過）
- [x] 全モジュールの mod.rs が空で存在する
- [x] テストヘルパーが定義されている
