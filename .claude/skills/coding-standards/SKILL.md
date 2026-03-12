---
name: coding-standards
description: Rust 開発のための汎用コーディング標準、ベストプラクティス、パターン。
---

# コーディング標準とベストプラクティス

Rust プロジェクトに適用するコーディング標準。

## コード品質の原則

### 1. 可読性第一
- コードは書くより読まれる回数の方が多い
- 明確な変数名と関数名
- コメントよりも自己文書化コードを優先
- 一貫したフォーマット（`cargo fmt`）

### 2. KISS（Keep It Simple, Stupid）
- 動作する最もシンプルな解決策
- 過度なエンジニアリングを避ける
- 早期の最適化をしない
- 理解しやすい > 賢いコード

### 3. DRY（Don't Repeat Yourself）
- 共通ロジックを関数に抽出
- トレイトで共通インターフェースを定義
- ジェネリクスで型の重複を削減
- コピペプログラミングを避ける

### 4. YAGNI（You Aren't Gonna Need It）
- 必要になる前に機能を作らない
- 推測的な汎化を避ける
- 必要な時にのみ複雑さを追加
- シンプルに始めて、必要に応じてリファクタリング

## Rust 命名規則

```rust
// 変数・関数: snake_case
let task_count = 0;
fn calculate_priority(task: &Task) -> Priority { }

// 型・トレイト・列挙型: CamelCase
struct TaskFilter { }
trait Repository { }
enum Priority { High, Medium, Low }

// 定数: SCREAMING_SNAKE_CASE
const MAX_TITLE_LENGTH: usize = 200;
static DEFAULT_PRIORITY: Priority = Priority::None;

// モジュール・ファイル: snake_case
mod task_service;
mod filter;
```

## エラーハンドリング

### `thiserror` でエラー型を定義
```rust
// ✅ 良い: 明示的なエラー型
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### `?` 演算子でエラー伝播
```rust
// ✅ 良い: ? でエラーを伝播
fn load_tasks(path: &Path) -> Result<Vec<Task>, AppError> {
    let content = std::fs::read_to_string(path)?;
    let tasks: Vec<Task> = serde_json::from_str(&content)?;
    Ok(tasks)
}

// ❌ 悪い: .unwrap() を本番コードで使用
fn load_tasks(path: &Path) -> Vec<Task> {
    let content = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&content).unwrap()
}
```

### `expect()` はコンテキスト付きで
```rust
// ✅ 良い: なぜパニックしないか説明
let config = load_config().expect("config.json must exist at startup");

// ❌ 悪い: コンテキストなし
let config = load_config().unwrap();
```

## 所有権とライフタイム

### 借用を優先
```rust
// ✅ 良い: 参照を受け取る
fn find_task(tasks: &[Task], id: &str) -> Option<&Task> {
    tasks.iter().find(|t| t.id.to_string().starts_with(id))
}

// ❌ 悪い: 不要な所有権の取得
fn find_task(tasks: Vec<Task>, id: String) -> Option<Task> {
    tasks.into_iter().find(|t| t.id.to_string().starts_with(&id))
}
```

### 不要な `.clone()` を避ける
```rust
// ✅ 良い: 参照で済む
fn display_title(task: &Task) {
    println!("{}", task.title);
}

// ❌ 悪い: 不要なクローン
fn display_title(task: &Task) {
    let title = task.title.clone();
    println!("{}", title);
}
```

### `&str` vs `String`
```rust
// ✅ 良い: パラメータは &str で受ける
fn search(query: &str) -> Vec<Task> { }

// ❌ 悪い: 不要に String を要求
fn search(query: String) -> Vec<Task> { }
```

## パターンマッチング

### 網羅的マッチ
```rust
// ✅ 良い: 全パターンを明示
match status {
    Status::Todo => "todo",
    Status::InProgress => "in_progress",
    Status::Done => "done",
    Status::Cancelled => "cancelled",
}

// ❌ 悪い: ワイルドカードで新しいバリアントを見逃す
match status {
    Status::Todo => "todo",
    _ => "other",
}
```

### `if let` で単一パターン
```rust
// ✅ 良い: 1つのケースだけ処理
if let Some(project) = &task.project {
    println!("Project: {project}");
}

// ❌ 冗長: 1ケースに match は過剰
match &task.project {
    Some(project) => println!("Project: {project}"),
    None => {},
}
```

## 構造体と列挙型

### ビルダーパターンよりも `Default`
```rust
// ✅ 良い: Default + 構造体更新構文
let filter = TaskFilter {
    project: Some("web".into()),
    ..Default::default()
};
```

### 列挙型で状態を表現
```rust
// ✅ 良い: 列挙型で不正な状態を排除
enum FormMode {
    New,
    Edit,
    Subtask,
}

// ❌ 悪い: 文字列で状態を管理
let mode: String = "new".into();
```

## イテレータ

```rust
// ✅ 良い: イテレータチェーン
let active_tasks: Vec<&Task> = tasks.iter()
    .filter(|t| t.status != Status::Done)
    .collect();

// ❌ 悪い: 手動ループ + push
let mut active_tasks = Vec::new();
for task in &tasks {
    if task.status != Status::Done {
        active_tasks.push(task);
    }
}
```

## ファイル構成

```
src/
├── main.rs                # エントリーポイント（CLI 引数パース）
├── lib.rs                 # ライブラリルート（任意）
├── error.rs               # AppError 定義
├── model/                 # データモデル層
│   ├── mod.rs
│   ├── task.rs
│   └── filter.rs
├── repository/            # データアクセス層
│   ├── mod.rs
│   └── json_store.rs
├── service/               # ビジネスロジック層
│   ├── mod.rs
│   ├── task_service.rs
│   └── sanitize.rs
├── cli/                   # CLI コマンドハンドラ
│   ├── mod.rs
│   ├── add.rs
│   └── list.rs
├── config/                # 設定管理
│   ├── mod.rs
│   └── settings.rs
└── tui/                   # TUI 層
    ├── mod.rs
    ├── app.rs
    └── pages/
```

### ファイルサイズ
- 通常 200〜400 行、最大 800 行
- 大きなファイルは機能別に分割

## テスト標準

### テスト構造（AAAパターン）
```rust
#[test]
fn filter_returns_matching_tasks() {
    // Arrange（準備）
    let tasks = create_sample_tasks();
    let filter = TaskFilter { project: Some("web".into()), ..Default::default() };

    // Act（実行）
    let result = filter_tasks(&tasks, &filter);

    // Assert（検証）
    assert_eq!(result.len(), 2);
}
```

### テストの命名
```rust
// ✅ 良い: 説明的なテスト名
#[test]
fn add_task_with_empty_title_returns_error() { }
#[test]
fn status_change_to_done_archives_task() { }
#[test]
fn filter_by_project_excludes_other_projects() { }

// ❌ 悪い: 曖昧なテスト名
#[test]
fn test_add() { }
#[test]
fn it_works() { }
```

## コードスメルの検出

### 1. 長い関数
```rust
// ❌ 悪い: 50行を超える関数 → 小さな関数に分割
// ✅ 良い: 各関数は1つの責務
fn process_task(task: &Task) -> Result<(), AppError> {
    let validated = validate(task)?;
    let sanitized = sanitize(validated)?;
    save(sanitized)
}
```

### 2. 深いネスト
```rust
// ❌ 悪い: 4段階以上のネスト
if let Some(task) = tasks.get(0) {
    if task.status == Status::Todo {
        if let Some(ref project) = task.project {
            if project == "web" {
                // ...
            }
        }
    }
}

// ✅ 良い: 早期リターンとガード節
let task = match tasks.get(0) {
    Some(t) => t,
    None => return,
};
if task.status != Status::Todo { return; }
let project = match &task.project {
    Some(p) => p,
    None => return,
};
if project != "web" { return; }
// ...
```

### 3. マジックナンバー
```rust
// ❌ 悪い
if title.len() > 200 { }

// ✅ 良い
const MAX_TITLE_LENGTH: usize = 200;
if title.len() > MAX_TITLE_LENGTH { }
```

## コメントとドキュメント

### `///` ドキュメントコメント（パブリックAPI用）
```rust
/// タスクをフィルタリングして返す。
///
/// `include_done` が false の場合、Done/Cancelled は除外される。
///
/// # Errors
///
/// ファイル読み込みに失敗した場合、`AppError::Io` を返す。
pub fn list_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>, AppError> { }
```

### `//` コメント（WHY を説明）
```rust
// ✅ 良い: なぜそうするのかを説明
// アトミック書き込み: 一時ファイルに書き込んでからリネーム
let tmp = path.with_extension("tmp");
std::fs::write(&tmp, content)?;
std::fs::rename(&tmp, path)?;

// ❌ 悪い: コードを繰り返しているだけ
// ファイルに書き込む
std::fs::write(&path, content)?;
```

---

**注意**: コード品質は交渉の余地がない。明確でメンテナンス可能なコードが、迅速な開発と自信を持ったリファクタリングを可能にする。`cargo clippy` と `cargo fmt` を常に実行すること。
