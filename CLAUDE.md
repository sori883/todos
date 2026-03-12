# CLAUDE.md

## Project

Rust CLI/TUI タスク管理ツール。clap (derive) + ratatui + crossterm。

## Build & Test

```bash
cargo build
cargo fmt --all -- --check \
  && cargo clippy --all-targets --all-features -- -D warnings \
  && RUSTFLAGS="-D warnings" cargo test --all-features
```

CI（`.github/workflows/ci.yml`）と同条件。push 前に必ず実行する。

## Rules & Skills

- `.claude/rules/dev-workflow.md` — 開発ワークフロー（CRITICAL）
- `.claude/rules/coding-style.md` — コーディングスタイル
- `.claude/skills/task-management/` — todos CLI によるタスク管理手順
- `.claude/skills/tdd-workflow/` — TDD ワークフロー
- `.claude/skills/e2e-test/` — E2E テスト生成
