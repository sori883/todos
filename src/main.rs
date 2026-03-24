use std::process;

use clap::{Parser, Subcommand, ValueEnum};

use todos::cli;
use todos::config::paths;
use todos::config::paths::db_path;
use todos::config::settings::Settings;
use todos::model::filter::TaskFilter;
use todos::model::task::{CreatedBy, Priority, Status};
use todos::service::task_service::TaskService;
use todos::store::sqlite_store::SqliteStore;
use todos::tui;

const COMMANDS_REFERENCE: &str = "\
\x1b[1;4mCommands Reference:\x1b[0m

  \x1b[1minit\x1b[0m [--force]
      プロジェクトを初期化（--force で上書き）

  \x1b[1madd\x1b[0m <TITLE> [-d <content>] [-p <priority>] [-c <created-by>]
                  [-l <label>] [-P <project>] [--parent <id>]
      タスクを追加。priority: none|low|medium|high|critical

  \x1b[1mshow\x1b[0m <ID>
      タスクの詳細を表示（ID は前方一致）

  \x1b[1mlist\x1b[0m (ls) [-s <status>] [-p <priority>] [-c <created-by>]
                [-l <label>] [-P <project>] [-q <query>]
                [--sort <field>] [--reverse] [--limit <n>] [--all] [--flat]
      タスク一覧を表示。sort: priority|created_at|updated_at|title

  \x1b[1medit\x1b[0m <ID> [-T <title>] [-d <content>] [-p <priority>]
                [-l <label>] [-P <project>] [--parent <id|none>]
      タスクを編集

  \x1b[1mstatus\x1b[0m <ID> <STATUS>
      ステータス変更。status: todo|in_progress|done|cancelled
      アーカイブ済みタスクも指定可能（done/cancelled 以外に変更すると復元）

  \x1b[1mdelete\x1b[0m (rm) <ID>
      タスクを削除

  \x1b[1msearch\x1b[0m <QUERY> [-s <status>] [-p <priority>] [-c <created-by>]
                   [-l <label>] [-P <project>]
      タスクを検索

  \x1b[1mstats\x1b[0m [-s <status>] [-p <priority>] [-c <created-by>]
                [-l <label>] [-P <project>]
      統計を表示

  \x1b[1mconfig\x1b[0m [--show] [--mode <vi|default>] [--icons <nerd|chars>]
                 [--max-title-length <n>] [--max-content-length <n>]
                 [--max-project-length <n>] [--reset]
      設定を管理

  \x1b[1marchive\x1b[0m [-s <status>] [-p <priority>] [-c <created-by>]
                  [-l <label>] [-P <project>] [-q <query>]
                  [--sort <field>] [--reverse] [--limit <n>]
      アーカイブ一覧を表示

  \x1b[1mbatch\x1b[0m
      一括操作（stdin から JSON）

\x1b[2m各コマンドの詳細: todos <command> --help\x1b[0m";

#[derive(Parser)]
#[command(
    name = "todos",
    about = "AI-human collaborative task manager",
    after_long_help = COMMANDS_REFERENCE
)]
struct Cli {
    /// データディレクトリのパスを指定
    #[arg(long = "data-dir", global = true)]
    data_dir: Option<std::path::PathBuf>,

    /// 出力フォーマット
    #[arg(long, default_value = "text", global = true)]
    format: OutputFormat,

    /// 確認プロンプトをスキップ
    #[arg(long, global = true)]
    yes: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Json => write!(f, "json"),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// プロジェクトを初期化
    Init {
        /// 既存データを上書き
        #[arg(long)]
        force: bool,
    },
    /// タスクを追加
    Add {
        /// タスクのタイトル
        title: String,

        /// 内容
        #[arg(short = 'd', long)]
        content: Option<String>,

        /// 優先度: none, low, medium, high, critical
        #[arg(short, long, default_value = "none")]
        priority: PriorityArg,

        /// 作成者: human, ai
        #[arg(short, long, default_value = "human")]
        created_by: CreatedByArg,

        /// 作業種別ラベル
        #[arg(short, long)]
        label: Option<String>,

        /// プロジェクト名
        #[arg(short = 'P', long)]
        project: Option<String>,

        /// 親タスク ID（前方一致）
        #[arg(long)]
        parent: Option<String>,
    },
    /// タスクの詳細を表示
    Show {
        /// タスク ID（前方一致）
        id: String,
    },
    /// タスク一覧を表示
    #[command(alias = "ls")]
    List {
        /// ステータスで絞り込み: todo, in_progress, done, cancelled
        #[arg(short = 's', long)]
        status: Option<StatusArg>,

        /// 優先度で絞り込み
        #[arg(short = 'p', long)]
        priority: Option<PriorityArg>,

        /// 作成者で絞り込み: human, ai
        #[arg(short = 'c', long)]
        created_by: Option<CreatedByArg>,

        /// ラベルで絞り込み
        #[arg(short = 'l', long)]
        label: Option<String>,

        /// プロジェクトで絞り込み
        #[arg(short = 'P', long)]
        project: Option<String>,

        /// 検索クエリ
        #[arg(short = 'q', long)]
        query: Option<String>,

        /// ソートフィールド: priority, created_at, updated_at, title
        #[arg(long, default_value = "created_at")]
        sort: String,

        /// ソート順を反転
        #[arg(long)]
        reverse: bool,

        /// 表示件数制限
        #[arg(long)]
        limit: Option<usize>,

        /// done/cancelled を含む
        #[arg(long)]
        all: bool,

        /// フラット表示（ツリー無効）
        #[arg(long)]
        flat: bool,
    },
    /// タスクを編集
    Edit {
        /// タスク ID（前方一致）
        id: String,

        /// 新しいタイトル
        #[arg(short = 'T', long)]
        title: Option<String>,

        /// 新しい内容
        #[arg(short = 'd', long)]
        content: Option<String>,

        /// 新しい優先度
        #[arg(short = 'p', long)]
        priority: Option<PriorityArg>,

        /// 新しいラベル
        #[arg(short = 'l', long)]
        label: Option<String>,

        /// 新しいプロジェクト
        #[arg(short = 'P', long)]
        project: Option<String>,

        /// 新しい親タスク ID（"none" で解除）
        #[arg(long)]
        parent: Option<String>,
    },
    /// タスクのステータスを変更（アーカイブ済みタスクも指定可能。done/cancelled 以外に変更すると復元される）
    Status {
        /// タスク ID（前方一致）
        id: String,
        /// 新しいステータス
        status: String,
    },
    /// タスクを削除
    #[command(alias = "rm")]
    Delete {
        /// タスク ID（前方一致）
        id: String,
    },
    /// タスクを検索
    Search {
        /// 検索クエリ
        query: String,

        /// ラベルで絞り込み
        #[arg(short = 'l', long)]
        label: Option<String>,

        /// プロジェクトで絞り込み
        #[arg(short = 'P', long)]
        project: Option<String>,

        /// 作成者で絞り込み: human, ai
        #[arg(short = 'c', long)]
        created_by: Option<CreatedByArg>,

        /// 優先度で絞り込み
        #[arg(short = 'p', long)]
        priority: Option<PriorityArg>,

        /// ステータスで絞り込み
        #[arg(short = 's', long)]
        status: Option<StatusArg>,
    },
    /// 統計を表示
    Stats {
        /// プロジェクトで絞り込み
        #[arg(short = 'P', long)]
        project: Option<String>,

        /// ラベルで絞り込み
        #[arg(short = 'l', long)]
        label: Option<String>,

        /// 作成者で絞り込み
        #[arg(short = 'c', long)]
        created_by: Option<CreatedByArg>,

        /// 優先度で絞り込み
        #[arg(short = 'p', long)]
        priority: Option<PriorityArg>,

        /// ステータスで絞り込み
        #[arg(short = 's', long)]
        status: Option<StatusArg>,
    },
    /// 設定を管理
    Config {
        /// 現在の設定を表示
        #[arg(long)]
        show: bool,

        /// キーバインドモード: vi, default
        #[arg(long)]
        mode: Option<String>,

        /// アイコンスタイル: nerd, chars
        #[arg(long)]
        icons: Option<String>,

        /// タイトルの最大文字数
        #[arg(long)]
        max_title_length: Option<usize>,

        /// コンテンツの最大文字数
        #[arg(long)]
        max_content_length: Option<usize>,

        /// プロジェクト名の最大文字数
        #[arg(long)]
        max_project_length: Option<usize>,

        /// 設定をリセット
        #[arg(long)]
        reset: bool,
    },
    /// アーカイブ一覧を表示
    Archive {
        /// ステータスで絞り込み: done, cancelled
        #[arg(short = 's', long)]
        status: Option<StatusArg>,

        /// 優先度で絞り込み
        #[arg(short = 'p', long)]
        priority: Option<PriorityArg>,

        /// 作成者で絞り込み: human, ai
        #[arg(short = 'c', long)]
        created_by: Option<CreatedByArg>,

        /// ラベルで絞り込み
        #[arg(short = 'l', long)]
        label: Option<String>,

        /// プロジェクトで絞り込み
        #[arg(short = 'P', long)]
        project: Option<String>,

        /// 検索クエリ
        #[arg(short = 'q', long)]
        query: Option<String>,

        /// ソートフィールド: priority, created_at, updated_at, title
        #[arg(long, default_value = "created_at")]
        sort: String,

        /// ソート順を反転
        #[arg(long)]
        reverse: bool,

        /// 表示件数制限
        #[arg(long)]
        limit: Option<usize>,
    },
    /// 一括操作（stdin から JSON）
    Batch,
}

#[derive(Debug, Clone, ValueEnum)]
enum PriorityArg {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl From<PriorityArg> for Priority {
    fn from(p: PriorityArg) -> Self {
        match p {
            PriorityArg::None => Priority::None,
            PriorityArg::Low => Priority::Low,
            PriorityArg::Medium => Priority::Medium,
            PriorityArg::High => Priority::High,
            PriorityArg::Critical => Priority::Critical,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum CreatedByArg {
    Human,
    Ai,
}

impl From<CreatedByArg> for CreatedBy {
    fn from(c: CreatedByArg) -> Self {
        match c {
            CreatedByArg::Human => CreatedBy::Human,
            CreatedByArg::Ai => CreatedBy::Ai,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum StatusArg {
    Todo,
    #[value(name = "in_progress")]
    InProgress,
    Done,
    Cancelled,
}

impl From<StatusArg> for Status {
    fn from(s: StatusArg) -> Self {
        match s {
            StatusArg::Todo => Status::Todo,
            StatusArg::InProgress => Status::InProgress,
            StatusArg::Done => Status::Done,
            StatusArg::Cancelled => Status::Cancelled,
        }
    }
}

/// Load the locale from settings. Returns default "ja" if settings cannot be loaded.
fn load_locale(data_dir: &std::path::Path) -> String {
    Settings::load(data_dir)
        .map(|s| s.locale)
        .unwrap_or_else(|_| "ja".to_string())
}

fn main() {
    let cli_args = Cli::parse();
    let format = cli_args.format.to_string();

    let result = run(cli_args, &format);

    if let Err(e) = result {
        cli::output::print_error(&e.to_string(), &format);
        process::exit(1);
    }
}

fn run(cli_args: Cli, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cli_args.command {
        None => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let service = TaskService::new(store, settings, archive_store);
            tui::run_tui(service, db)?;
            Ok(())
        }
        Some(Commands::Init { force }) => {
            let data_dir = paths::init_data_dir(cli_args.data_dir.as_deref());
            let locale = load_locale(&data_dir);
            cli::init::run(&data_dir, force, format, &locale)?;
            Ok(())
        }
        Some(Commands::Add {
            title,
            content,
            priority,
            created_by,
            label,
            project,
            parent,
        }) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let locale = settings.locale.clone();
            let service = TaskService::new(store, settings, archive_store);

            let params = cli::add::AddParams {
                title,
                content,
                priority: priority.into(),
                created_by: created_by.into(),
                label,
                project,
                parent,
            };

            cli::add::run(&service, params, format, &locale)?;
            Ok(())
        }
        Some(Commands::Show { id }) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let service = TaskService::new(store, settings, archive_store);

            cli::show::run(&service, &id, format)?;
            Ok(())
        }
        Some(Commands::List {
            status,
            priority,
            created_by,
            label,
            project,
            query,
            sort,
            reverse,
            limit,
            all,
            flat,
        }) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let service = TaskService::new(store, settings, archive_store);

            let status_filter: Option<Status> = status.map(|s| s.into());
            let has_status_filter = status_filter.is_some();

            let filter = TaskFilter {
                status: status_filter,
                priority: priority.map(|p| p.into()),
                created_by: created_by.map(|c| c.into()),
                label,
                project,
                parent_id: None,
                include_done: all || has_status_filter,
                include_cancelled: all || has_status_filter,
            };

            // If query is provided, use search_tasks
            if let Some(ref q) = query {
                let tasks = service.search_tasks(q, &filter)?;
                let count = tasks.len();
                let data = serde_json::json!({
                    "count": count,
                    "tasks": tasks,
                });
                if format == "json" {
                    let response = cli::output::CliResponse::success(data);
                    cli::output::print_response(&response, format);
                } else {
                    let response = cli::output::CliResponse::success_with_message(
                        data,
                        format!("Found {} task(s)", count),
                    );
                    cli::output::print_response(&response, format);
                }
                return Ok(());
            }

            let params = cli::list::ListParams {
                filter,
                sort,
                reverse,
                limit,
                flat,
            };

            cli::list::run(&service, params, format)?;
            Ok(())
        }
        Some(Commands::Edit {
            id,
            title,
            content,
            priority,
            label,
            project,
            parent,
        }) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let locale = settings.locale.clone();
            let service = TaskService::new(store, settings, archive_store);

            let params = cli::edit::EditParams {
                id,
                title,
                content,
                priority: priority.map(|p| p.into()),
                label,
                project,
                parent,
            };

            cli::edit::run(&service, params, format, &locale)?;
            Ok(())
        }
        Some(Commands::Status { id, status }) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let locale = settings.locale.clone();
            let service = TaskService::new(store, settings, archive_store);

            cli::status::run(&service, &id, &status, format, &locale)?;
            Ok(())
        }
        Some(Commands::Delete { id }) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let locale = settings.locale.clone();
            let service = TaskService::new(store, settings, archive_store);

            cli::delete::run(&service, &id, cli_args.yes, format, &locale)?;
            Ok(())
        }
        Some(Commands::Search {
            query,
            label,
            project,
            created_by,
            priority,
            status,
        }) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let service = TaskService::new(store, settings, archive_store);

            let status_filter: Option<Status> = status.map(|s| s.into());
            let has_status_filter = status_filter.is_some();

            let filter = TaskFilter {
                status: status_filter,
                priority: priority.map(|p| p.into()),
                created_by: created_by.map(|c| c.into()),
                label,
                project,
                parent_id: None,
                include_done: has_status_filter,
                include_cancelled: has_status_filter,
            };

            let tasks = service.search_tasks(&query, &filter)?;
            let count = tasks.len();
            let data = serde_json::json!({
                "count": count,
                "tasks": tasks,
            });

            if format == "json" {
                let response = cli::output::CliResponse::success(data);
                cli::output::print_response(&response, format);
            } else {
                let response = cli::output::CliResponse::success_with_message(
                    data,
                    format!("Found {} task(s)", count),
                );
                cli::output::print_response(&response, format);
            }
            Ok(())
        }
        Some(Commands::Stats {
            project,
            label,
            created_by,
            priority,
            status,
        }) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let service = TaskService::new(store, settings, archive_store);

            let filter = TaskFilter {
                status: status.map(|s| s.into()),
                priority: priority.map(|p| p.into()),
                created_by: created_by.map(|c| c.into()),
                label,
                project,
                parent_id: None,
                include_done: true,
                include_cancelled: true,
            };

            cli::stats::run(&service, &filter, format)?;
            Ok(())
        }
        Some(Commands::Config {
            show,
            mode,
            icons,
            max_title_length,
            max_content_length,
            max_project_length,
            reset,
        }) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());

            let params = cli::config::ConfigParams {
                show,
                mode,
                icons,
                max_title_length,
                max_content_length,
                max_project_length,
                reset,
                yes: cli_args.yes,
            };

            cli::config::run(&data_dir, params, format)?;
            Ok(())
        }
        Some(Commands::Archive {
            status,
            priority,
            created_by,
            label,
            project,
            query,
            sort,
            reverse,
            limit,
        }) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let service = TaskService::new(store, settings, archive_store);

            let filter = TaskFilter {
                status: status.map(|s| s.into()),
                priority: priority.map(|p| p.into()),
                created_by: created_by.map(|c| c.into()),
                label,
                project,
                parent_id: None,
                include_done: true,
                include_cancelled: true,
            };

            let params = cli::archive::ArchiveParams {
                filter,
                query,
                sort,
                reverse,
                limit,
            };

            cli::archive::run(&service, params, format)?;
            Ok(())
        }
        Some(Commands::Batch) => {
            let data_dir = paths::resolve_data_dir(cli_args.data_dir.as_deref());
            let db = db_path(&data_dir);
            let conn = SqliteStore::open(&db)?;
            let store = SqliteStore::new(conn.clone(), "tasks")?;
            let archive_store = SqliteStore::new(conn, "archive")?;
            let settings = Settings::load(&data_dir)?;
            let service = TaskService::new(store, settings, archive_store);

            cli::batch::run(&service, format)?;
            Ok(())
        }
    }
}
