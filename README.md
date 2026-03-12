# todos

バイブコーディング用のタスク管理ツール

## 機能

- **CLI / TUI** — コマンドラインとターミナルUI（ratatui）の2つのインターフェース
- **親子タスク** — 2階層のツリー構造でサブタスク管理
- **プロジェクト・ラベル** — プロジェクト別のグループ化、ラベルによる分類
- **自動アーカイブ** — Done/Cancelled タスクは自動で archive.json に移動
- **JSON 出力** — `--format json` で全コマンドが機械可読な出力に対応
- **一括操作** — `batch` コマンドで stdin から JSON による複数操作を一度に実行
- **i18n** — 日本語/英語対応
- **設定可能** — キーバインド、アイコン、文字数制限などをカスタマイズ

## インストール

```bash
cargo install --git https://github.com/sori883/todos
```

## 使い方

```bash
# 初期化
todos init

# TUI 起動（コマンド省略時）
todos

# タスク追加
todos add "機能を実装する" --priority high --label feature --project myapp

# サブタスク追加
todos add "設計" --parent <ID_PREFIX>

# 一覧表示
todos list
todos list -P myapp # プロジェクトで絞り込み
todos list --format json # JSON 出力

# ステータス変更
todos status <ID_PREFIX> in_progress
todos status <ID_PREFIX> done # 自動アーカイブ
```

## データ保存場所

- **プロジェクト**: プロジェクトルートで `todos init` → カレントディレクトリに `.todos/` を作成
- **グローバル**: `~/.todos/`（どこにも `.todos/` がない場合、ホームに自動作成）
- **明示指定**: `--data-dir <PATH>` → `<PATH>/.todos/`

探索ロジック: カレントディレクトリから親を辿り `.todos/` を探す。ホームディレクトリで停止。

| ファイル | 内容 |
|---------|------|
| `.todos/tasks.json` | アクティブなタスク |
| `.todos/archive.json` | Done/Cancelled タスク |
| `.todos/settings.json` | ユーザー設定 |

## CLIコマンド

`todos --help` で全コマンドとオプションを確認できます。

| コマンド | 説明 |
|---------|------|
| `init` | プロジェクトを初期化 |
| `add` | タスクを追加 |
| `show` | タスクの詳細を表示 |
| `list` (`ls`) | タスク一覧を表示 |
| `edit` | タスクを編集 |
| `status` | ステータスを変更 |
| `delete` (`rm`) | タスクを削除 |
| `search` | タスクを検索 |
| `stats` | 統計を表示 |
| `config` | 設定を管理 |
| `archive` | アーカイブ一覧を表示 |
| `batch` | 一括操作（stdin から JSON） |

## TUIキーバインド

| キー | 操作 |
|------|------|
| `j` / `k` | カーソル移動 |
| `h` / `l` | プロジェクトタブ切替 |
| `n` | 新規タスク |
| `e` | 編集 |
| `s` | サブタスク追加 |
| `Space` | ステータス切替 |
| `x` | キャンセル |
| `d` | 削除 |
| `c` | 完了タスクの表示切替 |
| `q` | 終了 |

## 併用おすすめClaude Code設定

`.claude/` 配下に、todos を使った開発ワークフローのためのルールとスキルを同梱しています。

| 種類 | 名前 | 内容 |
|------|------|------|
| ルール | `dev-workflow.md` | todos CLI でタスク管理しながら開発するワークフロー |
| スキル | `/task-management` | todos CLI のコマンドテンプレートとフェーズ実行手順 |
| スキル | `/task-verification` | todos のタスク仕様と実装の突合検証 |

## ライセンス

MIT
