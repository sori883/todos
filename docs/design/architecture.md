# 設計ドキュメント: todos -- AI + 人間 協働タスク管理ツール

## 概要

Rust 製の CLI/TUI タスク管理ツール。AI エージェント（Claude Code 等）と人間が同じタスクデータを共有して協働作業することを前提に設計する。CLI は AI が操作する第一級インターフェースとして機械可読な JSON 出力を標準サポートし、TUI は人間が直感的にタスクを管理するためのインターフェースとして提供する。

CLI/TUI 共存、JSON ファイルベースの永続化、ratatui + crossterm による TUI をベースに、AI 協働に必要な拡張を組み込む。

---

## 1. データモデル

### Task 構造体

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type TaskId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub status: Status,
    pub priority: Priority,
    pub created_by: CreatedBy,
    /// 作業種別（設定ファイルの labels リストでバリデーション）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// マイクロサービス名（バリデーションなし。任意の文字列を許可）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// 親タスク ID（None = ルートタスク、Some = サブタスク）。2階層まで
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<TaskId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Todo,
    InProgress,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    #[default]
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CreatedBy {
    Human,
    Ai,
}
```

### TaskFilter 構造体

```rust
#[derive(Debug, Default)]
pub struct TaskFilter {
    pub status: Option<Vec<Status>>,
    pub priority: Option<Vec<Priority>>,
    pub created_by: Option<CreatedBy>,
    pub label: Option<Vec<String>>,
    pub project: Option<Vec<String>>,
    pub parent_id: Option<Option<TaskId>>,  // None=フィルタなし, Some(None)=ルートのみ, Some(Some(id))=特定の親の子のみ
    pub sort_by: SortField,
    pub sort_reverse: bool,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum SortField {
    #[default]
    Priority,
    CreatedAt,
    UpdatedAt,
    Title,
}
```

### Stats 構造体

```rust
#[derive(Debug, Serialize)]
pub struct Stats {
    pub total: usize,
    pub by_status: HashMap<Status, usize>,
    pub by_priority: HashMap<Priority, usize>,
    pub by_creator: HashMap<CreatedBy, usize>,
    pub by_label: HashMap<String, usize>,
    pub by_project: HashMap<String, usize>,
}
```

### 設計判断

| 判断 | 理由 |
|---|---|
| UUID v4 を使用 | AI と人間が独立してタスクを作成する場合に ID 衝突を防ぐ |
| `Status` を 4 段階に | bool（完了/未完了）では作業中の状態を表現できない |
| `Priority` を 5 段階に | AI がタスクの重要度を判断して優先順位付けするために必要 |
| `CreatedBy` フィールド | AI が自動生成したタスクと人間が手動で作ったタスクを区別。作成者はタスクの起源を示すため変更不可 |
| `label` を String + バリデーションに | ビルトイン値はハードコード、ユーザーは設定ファイルで追加のみ可能。許可リスト外はエラー |
| `project` を String + バリデーションなしに | 任意の文字列を許可。TUI セレクタや `stats` での集計用にリストを管理するが、新規プロジェクト名の入力を制限しない |
| UTC で保存 | タイムゾーン問題を回避。表示時にローカル時間に変換 |
| ロケール対応 | 初期は日本語(`ja`)。`i18n/` モジュールで UI テキスト・日付フォーマットを切替可能な設計にし、将来の多言語化に備える |
| ID 前方一致は最低 4 文字 | UUID の衝突リスクを抑えつつ、短い入力で操作可能にする |
| 親子タスクは2階層まで | 親→子の関係のみ。孫タスクは許可しない。マイクロサービス開発で「機能→サブタスク」の分割に十分 |
| 親子のステータスは独立 | 子タスクが全て `done` でも親は自動変更しない。AI/人間が明示的に管理する |

### 親子タスク

タスクは親→子の2階層構造を持てる。

| 項目 | ルール |
|---|---|
| 階層制限 | 2階層まで（親→子）。子タスクに `parent_id` を持つタスクを子として追加しようとするとエラー |
| ステータス | 親と子は独立。子が全て `done` でも親は自動変更しない |
| 親の削除 | 親を削除すると子タスクも全て削除される（確認プロンプトで子の数を表示） |
| 子の project | 未指定時は親の `project` を継承する。明示指定時はその値を使用 |
| フィルタ | `list` のデフォルトは親子をツリー表示。`--flat` で階層を無視したフラット表示 |

### データ保存場所

カレントディレクトリから上方向に `.todos/` を探索し、最初に見つかったディレクトリを使う。探索はホームディレクトリ（`~`）を上限とし、それを超えた場合はグローバル（`~/.config/todos/`）にフォールバックする。

```
~/work/my-system/user-service/src/ で実行した場合の探索順:

1. ~/work/my-system/user-service/src/.todos/  ← ない
2. ~/work/my-system/user-service/.todos/      ← あればここを使う
3. ~/work/my-system/.todos/                   ← なければ次へ
4. ~/work/.todos/
5. ~/ (.todos/)                               ← ホームが探索上限
6. ~/.config/todos/                           ← 最終フォールバック（グローバル）
```

`/tmp/build/` のようなホーム外のディレクトリで実行した場合、`.todos/` が見つからなければ即座にグローバルにフォールバックする。

`todos init` で カレントディレクトリに `.todos/` を作成する。

```
.todos/
├── tasks.json       # アクティブなタスクデータ（todo, in_progress）
├── archive.json     # アーカイブ済みタスクデータ（done, cancelled）
└── settings.json    # 設定（省略時はグローバル設定を使用）
```

### データファイル形式

```json
{
  "version": 1,
  "tasks": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "API エンドポイントの実装",
      "content": "ユーザー認証の REST API を実装する",
      "status": "in_progress",
      "priority": "high",
      "created_by": "human",
      "label": "feature",
      "project": "user-service",
      "created_at": "2026-03-10T08:00:00Z",
      "updated_at": "2026-03-11T10:30:00Z"
    },
    {
      "id": "7c9e2200-a1b2-4c3d-8e9f-112233445566",
      "title": "ログインAPI実装",
      "status": "todo",
      "priority": "high",
      "created_by": "ai",
      "label": "feature",
      "project": "user-service",
      "parent_id": "550e8400-e29b-41d4-a716-446655440000",
      "created_at": "2026-03-10T09:00:00Z",
      "updated_at": "2026-03-10T09:00:00Z"
    }
  ]
}
```

トップレベルに `version` フィールドを置き、将来のスキーマ変更時にマイグレーションを実現する。

`status` が `done` のタスクは `completed_at` が必須。データ読み込み時に `done` かつ `completed_at` がない場合は `updated_at` の値で補完する。

### スキーママイグレーション

`version` フィールドによりスキーマの変更を管理する。

| 項目 | ルール |
|---|---|
| バージョン判定 | ファイル読み込み時に `version` を確認。現在の期待バージョンと一致しなければマイグレーションを実行 |
| マイグレーション方向 | 上位バージョンへの変換のみサポート（ダウングレード不可） |
| マイグレーション実装 | `store/schema.rs` に `fn migrate(data: Value, from: u32, to: u32) -> Result<Value>` を定義 |
| 自動バックアップ | マイグレーション実行前に `tasks.json.bak` を作成 |
| 未知のバージョン | 現在の期待バージョンより大きい場合はエラー（新しいバージョンのツールで作成されたデータ） |

### アーカイブ

タスクのステータスが `done` または `cancelled` に変更された際、そのタスク（およびサブタスク）は自動的に `archive.json` に移動される。逆に `todo` や `in_progress` に戻された場合は `tasks.json` に復元される。

| 項目 | ルール |
|---|---|
| 自動アーカイブ | `status` が `done` / `cancelled` に変更されたタスクとそのサブタスクを `archive.json` に移動 |
| 自動復元 | アーカイブ済みタスクの `status` が `todo` / `in_progress` に変更された場合、`tasks.json` に復元 |
| サブタスクの連動 | 親タスクのアーカイブ/復元時に子タスクも連動して移動 |
| CLI 参照 | `todos archive` コマンドでアーカイブ済みタスクを一覧表示 |
| TUI 参照 | `c` キーでアーカイブ済みタスクの表示を切替（トグル） |
| データファイル | `archive.json`（`tasks.json` と同じスキーマ） |

---

## 2. CLI コマンド体系

バイナリ名: `todos`

### コマンド一覧

```
todos [COMMAND]          # コマンドなしで TUI を起動
todos add                # タスクを追加
todos list (ls)          # タスク一覧を表示
todos show               # タスクの詳細を表示
todos edit               # タスクを編集
todos status             # タスクのステータスを変更
todos delete (rm)        # タスクを削除
todos search             # タスクを全文検索（list --query のエイリアス）
todos archive            # アーカイブ済みタスクを一覧表示
todos stats              # 統計情報を表示
todos config             # 設定を変更/表示
todos init               # データファイルを初期化
todos batch              # 標準入力から一括操作（AI 向け）
```

### グローバルオプション

```
--format <FORMAT>        # 出力フォーマット: text (default), json
--yes                    # 確認プロンプトをスキップ（AI 向け）
--global                 # グローバル（~/.config/todos/）を強制使用
--data-dir <PATH>        # データディレクトリを明示指定（探索をスキップ）
```

### ID 引数の共通ルール

`<ID>` を受け取るコマンド（`show`, `edit`, `status`, `delete`）は、UUID の前方一致で検索する（git のコミットハッシュと同じ操作感）。最低 4 文字が必要。一意に特定できない場合はエラー。

### 出力先ルール

| 出力種別 | 出力先 |
|---|---|
| 正常結果（text / json） | stdout |
| エラー（text モード） | stderr |
| エラー（json モード） | stdout に `{ "success": false, "error": "..." }` を出力 |
| 警告・デバッグ | stderr |

AI がパースするのは stdout のみ。json モードではエラーも stdout に JSON 形式で出力し、終了コードで成否を判定する。

### CLI JSON 出力ラッパー

全コマンドの JSON 出力は以下のラッパー構造を使用する。`cli/output.rs` で定義。

```rust
#[derive(Serialize)]
pub struct CliResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
```

- 成功時: `{ "success": true, "data": { ... } }`
- 失敗時: `{ "success": false, "error": "..." }`

コマンド固有のフィールド（`task`, `tasks`, `count` 等）は `data` の中に含まれる。

### 各コマンドの詳細

#### `todos add`

```
todos add <TITLE> [OPTIONS]

Arguments:
  <TITLE>                タスク名

Options:
  -d, --content <TEXT>          説明（content）
  -p, --priority <PRIORITY>     優先度: none (default), low, medium, high, critical
  -c, --created-by <BY>         作成者: human (default), ai
  -l, --label <LABEL>           作業種別: bug, feature, improvement, documentation, refactor, chore（+ extra_labels）
  -P, --project <PROJECT>       プロジェクト（マイクロサービス名、任意の文字列）
  --parent <ID>                 親タスク ID（前方一致）。指定するとサブタスクとして作成
```

`--parent` 指定時: 親タスクが既にサブタスクの場合はエラー（2階層制限）。`--project` 未指定なら親の `project` を継承。

新規タスクの `status` は常に `todo`。ステータスの指定が必要な場合は `add` 後に `status` コマンドで変更する（ステータス変更の副作用を確実に発生させるため）。

出力例（json）:
```json
{ "success": true, "data": { "task": { ... } } }
```

#### `todos list` (エイリアス: `ls`)

```
todos list [OPTIONS]

Options:
  -s, --status <STATUS>         ステータスでフィルタ（複数指定可: -s todo -s in_progress）
  -p, --priority <PRIORITY>     優先度でフィルタ（複数指定可: -p high -p critical）
  -c, --created-by <BY>         作成者でフィルタ: human, ai
  -l, --label <LABEL>           作業種別でフィルタ（複数指定可: -l bug -l feature）
  -P, --project <PROJECT>       プロジェクトでフィルタ（複数指定可: -P user-service -P order-service）
  -q, --query <QUERY>           テキスト検索（title と content を対象）
  --sort <FIELD>                ソート: priority (default), created_at, updated_at, title
  --reverse                     逆順ソート
  --limit <N>                   表示件数制限
  --all                         done/cancelled を含む全タスクを表示
  --flat                        階層を無視してフラット表示
```

`-s` 未指定かつ `--all` なしの場合、`default_view.show_completed` 設定に従う（デフォルト `false` = `todo` と `in_progress` のみ表示）。`-s` を明示指定した場合は設定に関わらずその値でフィルタする。

デフォルトはツリー表示（親の下に子タスクをインデント表示）。`--flat` でフラット表示に切り替え。ソートは親タスクに対して適用し、子タスクは親の直後に `created_at` 順で表示する。

出力例（text、ツリー表示）:
```
 ID       STATUS       PRI    TITLE                       PROJECT         LABEL    BY
 550e84.. in_progress  high   API エンドポイント実装       user-service    feature  human
   7c9e22.. todo       high     ログインAPI実装            user-service    feature  ai
   a3b1c8.. todo       medium   トークン検証実装           user-service    feature  ai
 d4e5f6.. todo         medium テスト追加                   user-service    test     human
```

出力例（json）:
```json
{ "success": true, "data": { "tasks": [ ... ], "count": 3 } }
```

#### `todos show`

```
todos show <ID>
```

出力例（text、親タスクの場合）:
```
ID:          550e8400-e29b-41d4-a716-446655440000
Title:       API エンドポイントの実装
Content:     ユーザー認証の REST API を実装する
Status:      in_progress
Priority:    high
Created by:  human
Label:       feature
Project:     user-service
Created at:  2026年03月10日 17:00
Updated at:  2026年03月11日 19:30

Subtasks (2):
  7c9e22.. todo   high   ログインAPI実装
  a3b1c8.. todo   medium トークン検証実装
```

出力例（text、サブタスクの場合）:
```
ID:          7c9e2200-a1b2-4c3d-8e9f-112233445566
Title:       ログインAPI実装
Status:      todo
Priority:    high
Created by:  ai
Label:       feature
Project:     user-service
Parent:      550e84.. API エンドポイントの実装
Created at:  2026年03月10日 18:00
Updated at:  2026年03月10日 18:00
```

出力例（json）:
```json
{ "success": true, "data": { "task": { ... }, "subtasks": [ ... ] } }
```

`subtasks` は親タスクの場合のみ含まれる。サブタスクの場合は `parent` フィールドに親の概要を含む。

#### `todos edit`

```
todos edit <ID> [OPTIONS]

Options:
  -T, --title <TITLE>           タスク名を変更
  -d, --content <TEXT>          説明（content）を変更
  -p, --priority <PRIORITY>     優先度を変更
  -l, --label <LABEL>           作業種別を変更
  -P, --project <PROJECT>       プロジェクトを変更
  --parent <ID>                 親タスクを変更（"none" で親子関係を解除）
```

`created_by` は `edit` で変更できない。作成者はタスクの起源を示すフィールドであり、変更すると履歴の信頼性が損なわれるため。

`status` は `edit` で変更できない。ステータス変更には副作用（`completed_at` 自動設定、自動アーカイブ）が伴うため、`status` コマンドに一本化する。

オプションを1つも指定しなかった場合はエラーを返す: `"No fields specified to edit. Use -T, -d, -p, -l, -P, or --parent"`

#### `todos status`

```
todos status <ID> <STATUS>

Arguments:
  <ID>       タスク ID（前方一致、最低 4 文字）
  <STATUS>   todo, in_progress, done, cancelled
```

`done` / `cancelled` に変更時、タスク（およびサブタスク）は自動的に `archive.json` にアーカイブされる。`done` に変更時、`completed_at` が自動設定。`done` から他のステータスに戻した場合、`completed_at` はクリアされ、タスクは `tasks.json` に復元される。

#### `todos delete` (エイリアス: `rm`)

```
todos delete <ID>
```

確認プロンプトを表示する。グローバルオプション `--yes` でスキップ可能。

親タスクを削除する場合、子タスクも全て削除される。確認プロンプトに子タスクの数を表示する:
```
Delete task "API エンドポイントの実装" and 2 subtasks? [y/N]
```

#### `todos search` (= `todos list --query`)

```
todos search <QUERY> [OPTIONS]
```

`todos list --query <QUERY> [OPTIONS]` と同等。AI が直感的にコマンド名で操作できるようにエイリアスとして提供する。オプションは `list` と同一。

#### `todos archive`

```
todos archive [OPTIONS]

Options:
  -s, --status <STATUS>         ステータスでフィルタ（done, cancelled）
  -p, --priority <PRIORITY>     優先度でフィルタ（複数指定可）
  -l, --label <LABEL>           作業種別でフィルタ（複数指定可）
  -P, --project <PROJECT>       プロジェクトでフィルタ（複数指定可）
  -q, --query <QUERY>           テキスト検索（title と content を対象）
  --sort <FIELD>                ソート: priority (default), created_at, updated_at, title
  --reverse                     逆順ソート
  --limit <N>                   表示件数制限
```

アーカイブ済み（`done` / `cancelled`）のタスクを一覧表示する。フィルタオプションは `list` と同様。

#### `todos stats`

```
todos stats [OPTIONS]

Options:
  -s, --status <STATUS>         ステータスでフィルタ（複数指定可）
  -p, --priority <PRIORITY>     優先度でフィルタ（複数指定可）
  -c, --created-by <BY>         作成者でフィルタ
  -l, --label <LABEL>           作業種別でフィルタ（複数指定可）
  -P, --project <PROJECT>       プロジェクトでフィルタ（複数指定可）
```

出力例（json）:
```json
{
  "success": true,
  "data": {
    "total": 15,
    "by_status": { "todo": 8, "in_progress": 3, "done": 3, "cancelled": 1 },
    "by_priority": { "critical": 1, "high": 3, "medium": 5, "low": 4, "none": 2 },
    "by_creator": { "human": 10, "ai": 5 },
    "by_label": { "feature": 5, "bug": 4, "test": 3, "refactor": 2, "docs": 1 },
    "by_project": { "user-service": 8, "order-service": 4, "api-gateway": 3 }
  }
}
```

#### `todos config`

```
todos config [OPTIONS]

Options:
  --show     現在の設定を表示（ビルトイン値 + 設定ファイルの内容）
  --reset    設定をリセット
  --mode <MODE>    キーバインドモード: default, vi
  --icons <ICONS>  アイコン: chars, nerd
```

`--reset` の挙動:

| 条件 | 挙動 |
|---|---|
| `--reset`（ローカル設定あり） | ローカル設定ファイルを削除 |
| `--reset`（ローカル設定なし） | 「ローカル設定は存在しません」と表示 |
| `--reset --global` | グローバル設定ファイルをデフォルト値で上書き |

`extra_labels` / `extra_projects` / `locale` / `date_formats` / `colors` 等の詳細設定は、設定ファイルを直接編集する。`config --show` で現在の設定パスと内容を確認できる。

#### `todos init`

```
todos init [OPTIONS]

Options:
  --force    既存データを上書き
```

#### `todos batch`（AI 一括操作）

```
todos batch [OPTIONS]

Options:
  --format <FORMAT>    入出力フォーマット: json (default)
```

標準入力から JSON 配列を受け取り、複数のタスク操作を一括実行する。AI エージェントが大量のタスクを効率的に作成・更新するための専用コマンド。

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BatchAction {
    Add {
        title: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        priority: Priority,
        #[serde(default = "default_created_by")]
        created_by: CreatedBy,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        project: Option<String>,
        #[serde(default)]
        parent_id: Option<String>,  // 親タスク ID（前方一致）
    },
    Status {
        id: String,
        status: Status,
    },
    Edit {
        id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        priority: Option<Priority>,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        project: Option<String>,
    },
    Delete {
        id: String,
    },
}
```

入力例:
```json
[
  { "action": "add", "title": "API実装", "priority": "high", "created_by": "ai", "label": "feature", "project": "user-service" },
  { "action": "status", "id": "550e84", "status": "done" },
  { "action": "edit", "id": "7c9e22", "priority": "critical" },
  { "action": "delete", "id": "a3b1c8" }
]
```

出力例:
```json
{
  "success": true,
  "data": {
    "results": [
      { "action": "add", "success": true, "task": { ... } },
      { "action": "status", "success": true, "task": { ... } },
      { "action": "edit", "success": true, "task": { ... } },
      { "action": "delete", "success": false, "error": "Task not found: a3b1c8" }
    ],
    "summary": { "total": 4, "succeeded": 3, "failed": 1 }
  }
}
```

一括操作はトランザクションではない（個別に実行し、失敗したアクションは結果に含まれるが他のアクションには影響しない）。ファイルへの書き込みは全アクション実行後に1回のみ行う。

### CLI 設計方針

1. 全コマンドが `--format json` をサポート
2. 成功時は終了コード 0、失敗時は非 0
3. JSON 出力は常に `CliResponse<T>` ラッパー（`{ "success": bool, "data": ... }` 形式）
4. ID の前方一致（最低 4 文字）で長い UUID の入力を不要に
5. `--yes` をグローバルオプションとして確認プロンプトをスキップ（AI 向け）
6. エイリアス: `list` -> `ls`, `delete` -> `rm`, `search` -> `list --query`
7. フィルタオプション（`-s`, `-p`, `-l`, `-P`）は複数指定可。`-c` は単一指定（human or ai）
8. json モードではエラーも stdout に出力（AI のパース対象を stdout に統一）
9. ステータス変更は `status` コマンドに一本化（副作用の一貫性を保証）

---

## 3. TUI 画面構成

### 画面一覧

1. タスク一覧画面（メイン） -- 起動時のデフォルト
2. タスク詳細パネル -- 一覧画面の右側に表示
3. タスク作成/編集画面 -- フォーム入力
4. タスク削除確認画面 -- 確認ダイアログ
5. フィルタ/検索パネル -- 新規追加

### タスク一覧画面

```
┌─ Projects ─────────────────────────────────────────────────────────────┐
│ [All] | user-service | order-service | api-gateway                     │
└───────────────────────────────────────────────────────────────────────┘
┌─ Todos ──────────────────────────────────┬─ Details ──────────────────┐
│                                          │                            │
│   ● [H] API エンドポイント実装  🔄       │  API エンドポイントの実装   │
│     ○ [H] ログインAPI実装       [AI]     │                            │
│     ○ [M] トークン検証実装      [AI]     │  ユーザー認証の REST API   │
│   ○ [M] テスト追加              [AI]     │  を実装する                │
│   ○ [L] ドキュメント更新                 │                            │
│                                          │  Status: in_progress       │
│                                          │  Priority: high            │
│                                          │  Created by: human         │
│                                          │  Label: feature            │
│                                          │  Project: user-service     │
│                                          │  Subtasks: 2 (0 done)      │
│                                          │  Created: 2026-03-10 17:00 │
│                                          │  Updated: 2026-03-11 19:30 │
│                                          │                            │
├──────────────────────────────────────────┴────────────────────────────┤
│[n]ew [s]ub [e]dit [d]el [Space]toggle [x]cancel [/]search [?]help q:quit│
└──────────────────────────────────────────────────────────────────────┘
```

- ステータスアイコン: `●` in_progress, `○` todo, `✓` done, `✗` cancelled
- 優先度: `[C]` Critical, `[H]` High, `[M]` Medium, `[L]` Low, `[ ]` None
- AI 作成タスクは一覧に `[AI]` マーカーを表示し、背景色でも区別
- 詳細パネルに `content`, `created_at`, `updated_at` を表示。`completed_at` は `done` の場合のみ表示
- 親タスクの場合、詳細パネルに `Subtasks: N (M done)` を表示
- サブタスクは親の直下にインデント表示。カーソルで親子を区別なく移動可能

### タスク作成/編集フォーム

フォームはターミナル領域の 90% を占めるオーバーレイとして表示される。6 つのフィールドで構成される。

```
┌─ New Task ───────────────────────────────────────────────────────────┐
│                                                                       │
│  Title:       [ テキスト入力                                       ]  │
│  Priority:    < none | low | medium | high | critical >  ← 左右で選択 │
│  Label:       < (none) | bug | feature | ... >           ← 左右で選択 │
│  Project:     [ テキスト入力                                       ]  │
│  Parent:      < (none) | API エンドポイント... | ... >   ← 左右で選択 │
│  Content:     [ 複数行テキスト入力（Unicode 対応行折返し）          ]  │
│               [                                                     ]  │
│               [                                                     ]  │
│                                                                       │
│       Tab/S-Tab: navigate  Left/Right: select  Enter: save  Esc: cancel│
│                                          Alt+Enter: newline (content)  │
└──────────────────────────────────────────────────────────────────────┘
```

- テキスト入力フィールド: `title`, `project`, `content`
- セレクタフィールド: `priority`, `label`, `parent`（定義済みリストから左右キーで選択）
- `content` フィールドは複数行入力に対応。Unicode 文字幅を考慮した行折返し（line wrapping）を行う。`Alt+Enter` で改行を挿入
- `parent` のセレクタは、ルートタスク（`parent_id` が None のタスク）のみ表示
- `label` のセレクタは、ビルトイン + extra_labels の統合リスト
- 編集フォームに `status` フィールドは含まない（ステータス変更は一覧画面の Space/x キーで行う）

### TUI キーバインド

| キー | アクション |
|---|---|
| `q` | 終了 |
| `j` / `Down` | カーソル下 |
| `k` / `Up` | カーソル上 |
| `Space` | ステータストグル（todo -> in_progress -> done -> todo） |
| `x` | ステータスを cancelled に変更 |
| `n` | タスク作成 |
| `s` | サブタスク作成（カーソル位置のタスクを親にして作成フォームを開く） |
| `e` | タスク編集 |
| `d` | タスク削除 |
| `h` / `Left` | 前のプロジェクト |
| `l` / `Right` | 次のプロジェクト |
| `/` | 検索/フィルタ |
| `c` | アーカイブ済み（done/cancelled）タスクの表示切替 |
| `?` | ヘルプ |
| `Enter` | 保存 |
| `Esc` | 戻る |

TUI の `Space` トグルで `done` への遷移時も `TaskService.change_status()` を経由するため、自動アーカイブが発生する。ステータスバーに通知メッセージを表示して、ユーザーにアーカイブを知らせる。

### TUI データ同期

AI と人間が同時に操作する場合、TUI は外部からのファイル変更を検知する必要がある。

| 項目 | 方式 |
|---|---|
| 検知方法 | イベントループの各 tick（約 250ms）で `tasks.json` のファイル修正時刻（mtime）をチェック |
| 変更検知時 | ファイルを再読み込みし、TUI の表示を更新。現在のカーソル位置・フィルタ状態は可能な限り維持 |
| TUI 編集中の競合 | TUI がフォームを開いている間にファイルが変更された場合、保存時に再読み込みしてからマージを試みる。編集対象タスクが削除されていた場合はエラーを表示 |
| ファイル監視ライブラリ | 初期実装は mtime ポーリング。将来的に `notify` クレートへの移行を検討 |

---

## 4. ファイル構成

```
src/
├── main.rs                    # エントリポイント: 引数有無で CLI/TUI 分岐
├── lib.rs                     # モジュール宣言
├── error.rs                   # アプリケーション全体のエラー型定義
│
├── model/                     # データモデル層
│   ├── mod.rs
│   ├── task.rs                # Task, Status, Priority, CreatedBy
│   ├── filter.rs              # TaskFilter, SortField
│   └── stats.rs               # Stats 構造体
│
├── service/                   # ビジネスロジック層
│   ├── mod.rs
│   ├── task_service.rs        # TaskService（バリデーション、アーカイブ、検索）
│   └── sanitize.rs            # 入力サニタイズ（タイトル・content・project の長さ制限）
│
├── store/                     # データ永続化層
│   ├── mod.rs
│   ├── repository.rs          # TaskRepository トレイト定義
│   ├── json_store.rs          # JSON ファイルによる実装（アトミック書き込み、ファイルロック含む）
│   └── schema.rs              # データファイルスキーマ、バージョン管理、マイグレーション
│
├── cli/                       # CLI 層
│   ├── mod.rs                 # clap コマンド定義、ディスパッチ
│   ├── add.rs
│   ├── list.rs                # search エイリアスもここで処理
│   ├── show.rs
│   ├── edit.rs
│   ├── status.rs
│   ├── delete.rs
│   ├── archive.rs             # アーカイブ済みタスクの一覧表示
│   ├── stats.rs
│   ├── config.rs
│   ├── init.rs
│   ├── batch.rs               # AI 一括操作、BatchAction 定義
│   └── output.rs              # CliResponse<T>、出力フォーマッタ（text/json、stdout/stderr 使い分け）
│
├── tui/                       # TUI 層
│   ├── mod.rs                 # TUI エントリ、ターミナル初期化
│   ├── app.rs                 # TUI 状態管理（キーハンドリング含む）
│   ├── app_form.rs            # フォーム操作のキーハンドラ
│   ├── event.rs               # イベントハンドリング（mtime ポーリング含む）
│   ├── pages/
│   │   ├── mod.rs
│   │   ├── task_list.rs       # タスク一覧画面
│   │   ├── task_detail.rs     # タスク詳細パネル
│   │   ├── task_form.rs       # 作成/編集フォーム
│   │   ├── delete_confirm.rs  # 削除確認
│   │   └── filter_panel.rs    # フィルタ/検索
│   └── widgets/
│       ├── mod.rs
│       ├── status_badge.rs
│       ├── priority_badge.rs
│       └── help_bar.rs
│
├── config/                    # 設定管理
│   ├── mod.rs
│   ├── settings.rs            # Settings 構造体
│   ├── keybindings.rs         # キーバインド定義
│   ├── theme.rs               # テーマ（カラー、アイコン、Nerd Fonts マッピング）
│   └── paths.rs               # .todos/ 探索（ホームディレクトリ上限）、tasks_json_path、archive_json_path
│
└── i18n/                      # 多言語対応
    ├── mod.rs                 # ロケール読み込み、メッセージ取得（enum + match 方式）
    ├── ja.rs                  # 日本語（初期デフォルト）
    └── en.rs                  # 英語
```

---

## 5. 技術スタック

| 用途 | クレート | バージョン |
|---|---|---|
| CLI 引数パース | `clap` | 4.x (derive) |
| TUI 描画 | `ratatui` | 0.29+ |
| ターミナルイベント | `crossterm` | 0.29+ |
| 日時処理 | `chrono` | 0.4 (serde feature) |
| シリアライズ | `serde` + `serde_json` | 1.x |
| UUID 生成 | `uuid` | 1.x (v4, serde) |
| アプリケーションエラー | `anyhow` | 1.x |
| ライブラリエラー定義 | `thiserror` | 2.x |
| ファイルパス解決 | `dirs` | 5.x |
| ファイルロック | `fs2` | 0.4 |
| イテレータ拡張 | `itertools` | 0.13+ |
| 文字幅計算 | `unicode-width` | 0.2（ratatui 0.29+ と同一メジャーバージョン） |

### `anyhow` / `thiserror` の使い分け

| 境界 | 使用クレート | 理由 |
|---|---|---|
| `error.rs`（`AppError` 定義） | `thiserror` | 構造化されたエラー型を `#[derive(Error)]` で定義 |
| `main.rs`, `cli/`, `tui/` | `anyhow` | `anyhow::Result` でエラーを伝搬、`context()` でメッセージ付加 |
| `service/`, `store/` | `thiserror` で定義、`anyhow` で伝搬 | ライブラリ的なコードは具象エラーを返し、呼び出し元が `anyhow` でラップ |

### Ctrl+C ハンドリング

`crossterm` のイベント機構で `KeyCode::Char('c')` + `Ctrl` を検知するため、TUI では別途ライブラリ不要。CLI 実行中は Rust のデフォルト SIGINT ハンドリング（プロセス終了）で十分。

---

## 6. 設定

### ビルトイン（変更不可）

以下はソースコードにハードコードされ、設定ファイルでは変更・削除できない。

| 項目 | ビルトイン値 |
|---|---|
| labels | `bug`, `feature`, `improvement`, `documentation`, `refactor`, `chore` |
| projects | `index`（デフォルトプロジェクト） |

### ユーザー追加（設定ファイルで変更可）

`extra_labels` でユーザー独自のラベルを追加できる。ビルトイン labels + extra_labels が利用可能な全ラベルリストになる。ラベルリスト外の値は `InvalidLabel` エラー。

`extra_projects` でプロジェクトを事前定義できる。ビルトイン projects（`index`）+ タスクデータから自動抽出されたプロジェクト + extra_projects が、TUI セレクタや `stats` での集計に使われるプロジェクトリストになる。ただし `project` フィールドの値は任意の文字列を許可し、リスト外の新規プロジェクト名も CLI/TUI から入力可能（入力された値は次回以降のリストに自動反映される）。

### 設定ファイル

パス: `.todos/settings.json`（ローカル） または `~/.config/todos/settings.json`（グローバル）

ローカルの設定が存在する場合はグローバル設定をベースにローカル設定で上書きする（マージ）。

マージ戦略:
- スカラー値（文字列、数値、bool）: ローカルが優先（上書き）
- 配列（`extra_labels`, `extra_projects`）: 重複排除結合（グローバル + ローカルの和集合）。比較は**大文字小文字を区別**する。結果の順序はグローバル → ローカルの出現順（重複はグローバル側の位置を維持）
- オブジェクト（`date_formats`, `colors` 等）: フィールド単位でローカルが優先
- 配列からの除外: 現時点ではサポートしない。グローバルで定義した `extra_labels` をローカルで除外する機能は将来検討

設定ファイルは `PartialSettings` パターンで読み込む。すべてのフィールドが `Option` であり、指定された値のみがデフォルト値を上書きする。未指定のフィールドはデフォルト値が使われる。

```json
{
  "locale": "ja",
  "extra_labels": ["perf", "security"],
  "extra_projects": ["notification-service"],
  "icons": {
    "style": "chars"
  },
  "keybindings": {
    "mode": "default"
  },
  "max_title_length": 200,
  "max_content_length": 10000,
  "max_project_length": 100
}
```

### 設定可能な長さ制限

| 設定キー | デフォルト値 | 説明 |
|---|---|---|
| `max_title_length` | 200 | タスクタイトルの最大文字数 |
| `max_content_length` | 10,000 | タスク content の最大文字数 |
| `max_project_length` | 100 | プロジェクト名の最大文字数 |

### `default_view.show_completed` の適用範囲

`show_completed: false`（デフォルト）の場合:
- CLI `list`（`-s` 未指定かつ `--all` なし）: `todo` と `in_progress` のみ表示（アクティブタスクのみ）
- TUI 一覧画面の初期状態: アーカイブ済みタスク（`done` / `cancelled`）を非表示（`c` キーでアーカイブタスクの表示を切替）
- `show_completed` は `done` と `cancelled` の両方に適用される
- TUI で `c` キーを押すと、`archive.json` からアーカイブ済みタスクを読み込んで一覧に追加表示する

### アイコンスタイル

`icons.style` で文字セットを切り替える。`style` 以外の個別アイコン設定は不要（スタイルでセットが決まる）。

| アイコン | `chars`（デフォルト） | `nerd`（Nerd Fonts） |
|---|---|---|
| status_todo | `○` | `` |
| status_in_progress | `●` | `` |
| status_done | `✓` | `` |
| status_cancelled | `✗` | `` |

### locale と i18n の関係

| 項目 | 説明 |
|---|---|
| `locale` | UI テキスト（ヘルプ、エラーメッセージ、TUI ラベル）の言語を決定する |
| `date_formats` | 日付の表示/入力フォーマット。`locale` とは独立して設定可能。`locale` 変更時にデフォルト値が変わるが、明示的に `date_formats` が設定されていればそちらが優先 |
| 無効な `locale` | サポート外の値が指定された場合は `en` にフォールバックし、警告を出力 |

### i18n 実装方式

enum + match によるシンプルな静的メッセージ方式。外部ファイルや i18n クレートは使用しない。

```rust
pub enum Message {
    TaskCreated,
    TaskUpdated,
    TaskDeleted,
    TaskDeletedWithSubtasks(usize),
    StatusChanged(String),
    Initialized(String),
    TaskArchived,
    TaskArchivedWithSubtasks(usize),
    TaskRestored,
    TaskRestoredWithSubtasks(usize),
}

pub fn get_message(msg: Message, locale: &str) -> String {
    match locale {
        "ja" => ja::get(msg),
        "en" => en::get(msg),
        _ => en::get(msg), // fallback to en
    }
}
```

---

## 7. アーキテクチャ

```
                    ┌─────────────┐
                    │   main.rs   │
                    │  (分岐)      │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              ▼                         ▼
        ┌───────────┐            ┌───────────┐
        │   CLI     │            │   TUI     │
        │  (clap)   │            │ (ratatui) │
        └─────┬─────┘            └─────┬─────┘
              │                        │
              │    ┌───────────────┐   │
              └───►│ TaskService   │◄──┘
                   │ (ビジネス     │
                   │  ロジック)    │
                   └──┬───┬────┬──┘
                      │   │    │
             ┌────────▼┐ ┌▼──┐ ┌▼────────┐
             │TaskRepo │ │   │ │ Settings │
             │(trait)  │ │   │ │(config/) │
             └───┬───┬─┘ │   │ └──────────┘
                 │   │    │   │
          ┌──────▼┐ ┌▼────▼┐  │
          │JsonStore│ │JsonStore│
          │(tasks  │ │(archive│
          │ .json) │ │ .json) │
          └────────┘ └────────┘

        i18n モジュールは CLI/TUI から直接参照
```

### TaskService 層

CLI/TUI と Repository の間にサービス層を配置し、ビジネスロジックを集約する。

`TaskService` のメソッドは全て `&self` で定義する。内部で保持する `JsonStore` が `RefCell` による内部可変性を持つため、`&self` のまま書き込み操作が可能。

```rust
pub struct TaskService {
    store: JsonStore,          // アクティブタスク用（tasks.json）
    settings: Settings,
    archive_store: JsonStore,  // アーカイブ用（archive.json）
}

impl TaskService {
    /// タスク追加（label バリデーション、入力サニタイズ含む。project はバリデーションなし）
    pub fn add_task(&self, ...) -> Result<Task>;

    /// タスク編集（label バリデーション、入力サニタイズ、updated_at 自動更新含む）
    pub fn edit_task(&self, prefix: &str, ...) -> Result<Task>;

    /// ステータス変更（completed_at 管理、自動アーカイブ/復元含む）
    pub fn change_status(&self, id_prefix: &str, status: &str) -> Result<StatusChangeResult>;

    /// タスク削除（子タスクも連動削除）
    pub fn delete_task(&self, id_prefix: &str) -> Result<DeleteResult>;

    /// タスク取得（前方一致）
    pub fn get_task(&self, id_prefix: &str) -> Result<Task>;

    /// 一覧取得（フィルタ適用）
    pub fn list_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>>;

    /// ツリー構造で一覧取得（親の下に子をグループ化）
    pub fn list_tasks_tree(&self, filter: &TaskFilter) -> Result<Vec<Task>>;

    /// テキスト検索 + フィルタ
    pub fn search_tasks(&self, query: &str, filter: &TaskFilter) -> Result<Vec<Task>>;

    /// 統計情報
    pub fn stats(&self, filter: &TaskFilter) -> Result<Stats>;

    /// 一括操作（batch コマンド用）
    pub fn batch(&self, actions: Vec<serde_json::Value>) -> Result<BatchResult>;

    /// 子タスク一覧を取得
    pub fn get_subtasks(&self, parent_id: TaskId) -> Result<Vec<Task>>;

    /// アーカイブからタスク取得（前方一致）
    pub fn get_task_from_archive(&self, prefix: &str) -> Result<Task>;

    /// アーカイブ済みタスク一覧取得（フィルタ適用）
    pub fn list_archive(&self, filter: &TaskFilter) -> Result<Vec<Task>>;

    /// アーカイブ済みタスクのテキスト検索
    pub fn search_archive(&self, query: &str, filter: &TaskFilter) -> Result<Vec<Task>>;

    /// キャッシュ無効化（外部からのファイル変更検知時に使用）
    pub fn invalidate_cache(&self);
}
```

`StatusChangeResult` は、ステータス変更後のタスクと、アーカイブ処理の結果（アーカイブされたか、連動したサブタスク数）を含む。

```rust
pub struct StatusChangeResult {
    pub task: Task,
    pub archived: bool,          // アーカイブ/復元が発生したか
    pub archived_subtasks: usize, // 連動してアーカイブ/復元されたサブタスク数
}

pub struct DeleteResult {
    pub task: Task,
    pub deleted_subtasks: usize,
}
```

### TaskService の責務

| 責務 | 説明 |
|---|---|
| バリデーション | label が許可リストに含まれるか検証（project はバリデーションなし） |
| 入力サニタイズ | title, content, project の長さ制限チェック（`sanitize.rs`） |
| ID 前方一致解決 | prefix → 完全な TaskId への解決。短すぎ・一致なし・複数一致のエラー処理 |
| ステータス遷移ロジック | `completed_at` の設定/クリア |
| 自動アーカイブ | `done` / `cancelled` 遷移時にタスクとサブタスクを `archive.json` に移動 |
| 自動復元 | アーカイブ済みタスクを `todo` / `in_progress` に戻す際に `tasks.json` に復元 |
| テキスト検索 | title/content に対する部分一致検索（アクティブ・アーカイブ両方対応） |
| `updated_at` 管理 | 変更時に自動で現在時刻を設定 |
| 一括操作 | batch コマンドの各アクションを順次実行 |
| 親子タスク管理 | 2階層制限の検証、親削除時の子タスク連動削除、子の project 継承 |

### TaskRepository トレイト

```rust
pub trait TaskRepository {
    fn list(&self, filter: &TaskFilter) -> Result<Vec<Task>>;
    fn get(&self, id: TaskId) -> Result<Option<Task>>;
    fn get_by_prefix(&self, prefix: &str) -> Result<Vec<Task>>;
    fn create(&self, task: Task) -> Result<Task>;
    fn update(&self, task: Task) -> Result<Task>;
    fn delete(&self, id: TaskId) -> Result<Option<Task>>;
    fn get_children(&self, parent_id: TaskId) -> Result<Vec<Task>>;
    fn stats(&self, filter: &TaskFilter) -> Result<Stats>;
}
```

`get_by_prefix` は前方一致する全タスクを `Vec<Task>` で返す。曖昧性チェック（0件 → `TaskNotFound`、2件以上 → `AmbiguousId`、prefix が4文字未満 → `IdPrefixTooShort`）は `TaskService` 層が担当する。

`search` は `TaskService` 層に移動。Repository は純粋なデータアクセスに専念し、テキスト検索のロジックはサービス層が担当する。

### 内部可変性（Interior Mutability）

`TaskRepository` のメソッドは全て `&self` で定義する。`JsonStore` は内部で `RefCell` を使い、ファイルの読み書き状態を管理する。

```rust
pub struct JsonStore {
    path: PathBuf,
    data: RefCell<Option<TaskData>>,
}
```

`TaskService` は `store`（tasks.json）と `archive_store`（archive.json）の 2 つの `JsonStore` を保持する。これにより TUI の `App` 構造体が `TaskService` を直接保持し、イベントハンドラから borrowing の問題なくデータを操作できる。マルチスレッドが必要になった場合は `RefCell` → `Mutex` に置き換える。

### データ整合性

#### アトミック書き込み

データ破損を防ぐため、ファイル書き込みは write-to-temp-then-rename パターンを使用する。

```rust
use std::io::{BufWriter, Write};

// json_store.rs での書き込み処理
fn save(&self, data: &TaskData) -> Result<()> {
    let temp_path = self.path.with_extension("json.tmp");
    // 1. 一時ファイルに書き込み（BufWriter で参照を保持）
    let file = File::create(&temp_path)?;
    let mut writer = BufWriter::new(&file);
    serde_json::to_writer_pretty(&mut writer, data)?;
    writer.flush()?;
    // 2. fsync で確実にディスクに書き込み
    file.sync_all()?;
    // 3. アトミックにリネーム（POSIX では同一ファイルシステム上でアトミック）
    std::fs::rename(&temp_path, &self.path)?;
    Ok(())
}
```

| 項目 | 詳細 |
|---|---|
| 一時ファイル名 | `tasks.json.tmp`（同一ディレクトリ内、同一ファイルシステムを保証） |
| BufWriter | `serde_json::to_writer_pretty` に参照を渡し、`File` の所有権を保持。`flush()` 後に `sync_all()` を呼ぶ |
| fsync | rename 前に `sync_all()` でディスクへのフラッシュを保証 |
| 失敗時 | 一時ファイルが残る。次回書き込み時に上書きされる |
| リネーム | POSIX の `rename(2)` はアトミック。既存ファイルは安全に置き換わる |

#### ファイルロック

AI（CLI）と人間（TUI）が同時にアクセスする場合のデータ競合を防ぐ。

| 項目 | 詳細 |
|---|---|
| ロック方式 | `fs2` クレートの `FileExt::lock_shared()` / `lock_exclusive()` を使用 |
| ロックファイル | `tasks.json.lock`（別ファイル。データファイル自体はロックしない） |
| 読み取り | 共有ロック（`lock_shared`）: 複数の読み取りを許可 |
| 書き込み | 排他ロック（`lock_exclusive`）: 書き込み中は他の読み書きをブロック |
| タイムアウト | `try_lock_exclusive` / `try_lock_shared`（ノンブロッキング）を 100ms 間隔で最大 50 回リトライ（計 5 秒） |
| TUI の挙動 | 書き込みロック取得失敗時、ステータスバーに「保存待ち...」を表示し、バックグラウンドでリトライ |
| CLI の挙動 | ロック取得失敗時、即座にエラーを返す（`--yes` 時も同様） |
| ロック粒度 | ファイル単位。タスク単位のロックは行わない（JSON ファイル全体を読み書きするため） |

---

## 8. エラーハンドリング

```rust
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Task not found: '{0}'")]
    TaskNotFound(String),

    #[error("Ambiguous ID '{prefix}': {count} tasks match")]
    AmbiguousId { prefix: String, count: usize },

    #[error("ID prefix too short: '{0}' (minimum 4 characters)")]
    IdPrefixTooShort(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Invalid label: '{0}'")]
    InvalidLabel(String),

    #[error("No edit fields specified")]
    NoEditFields,

    #[error("Nesting too deep: subtasks cannot have children")]
    NestingTooDeep,

    #[error("Data file error: {0}")]
    DataFile(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("File lock error: {0}")]
    FileLock(String),

    #[error("Schema version {0} is newer than supported version {1}")]
    SchemaVersionTooNew(u32, u32),

    #[error("Schema migration failed: {0}")]
    SchemaMigration(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Already initialized: use --force to overwrite")]
    AlreadyInitialized,
}
```

JSON モード時のエラー出力:
```json
{ "success": false, "error": "Task not found: 550e84" }
```

---

## 9. 実装フェーズ

### フェーズ 1: 基盤（model + store + service）
1. `error.rs` -- エラー型定義
2. `model/task.rs` -- Task, Status, Priority, CreatedBy
3. `model/filter.rs` -- TaskFilter, SortField
4. `model/stats.rs` -- Stats 構造体
5. `store/schema.rs` -- データファイルスキーマ、マイグレーション
6. `store/repository.rs` -- TaskRepository トレイト
7. `store/json_store.rs` -- JSON ファイル実装（アトミック書き込み、ファイルロック含む）
8. `config/paths.rs` -- .todos/ 探索（ホームディレクトリ上限）、`archive_json_path`
9. `config/settings.rs` -- Settings 構造体、PartialSettings パターン、マージ処理
10. `service/sanitize.rs` -- 入力サニタイズ（長さ制限）
11. `service/task_service.rs` -- TaskService（バリデーション、アーカイブ）
12. ユニットテスト

### フェーズ 2: CLI
1. `cli/mod.rs` -- clap コマンド定義
2. `cli/output.rs` -- CliResponse<T>、出力フォーマッタ（stdout/stderr 使い分け）
3. 各サブコマンド実装（search は list --query のエイリアス）
4. `cli/archive.rs` -- アーカイブ済みタスクの一覧表示
5. `cli/batch.rs` -- AI 一括操作、BatchAction 定義
6. `main.rs` -- CLI 分岐
7. 統合テスト

### フェーズ 3: TUI
1. `tui/mod.rs` -- ターミナル初期化
2. `tui/app.rs` -- TUI 状態管理
3. `tui/event.rs` -- イベントハンドリング（mtime ポーリング含む）
4. `tui/pages/` -- 各画面
5. `tui/widgets/` -- 共通ウィジェット
6. `config/keybindings.rs`, `config/theme.rs` -- 型安全な構造体に置き換え
7. `main.rs` -- TUI 分岐を追加

### フェーズ 4: 仕上げ
1. `i18n/` -- 多言語対応（enum + match 方式）
2. エッジケースのテスト追加
3. パフォーマンス検証
4. エラーメッセージの改善

---

## 10. リスクと軽減策

| リスク | 軽減策 |
|---|---|
| JSON ファイルの同時書き込み | `fs2` によるアドバイザリロック + アトミック書き込み（write-to-temp-then-rename） |
| AI + TUI の同時アクセス | TUI は mtime ポーリングでファイル変更を検知、自動再読み込み |
| 書き込み中のクラッシュ | アトミック書き込みにより、中途半端な状態のファイルが残らない |
| 大量タスク時のパフォーマンス | 10,000 件程度を上限想定。超過時は将来 SQLite に移行 |
| UUID 前方一致の衝突 | 最低 4 文字を要求、複数マッチ時はエラー |
| TUI クラッシュ時のターミナル破壊 | panic フックでターミナルリストア処理を実行 |
| スキーマ変更 | `version` フィールドによるマイグレーション。変更前に自動バックアップ |
