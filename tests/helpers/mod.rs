use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

/// テスト用の一時ディレクトリを作成し、init を実行する
pub fn setup() -> TempDir {
    let dir = TempDir::new().unwrap();
    todos_cmd(dir.path()).args(["init"]).assert().success();
    dir
}

/// --data-dir 付きの todos コマンドを返す
pub fn todos_cmd(data_dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("todos").unwrap();
    cmd.arg("--data-dir").arg(data_dir);
    cmd
}

/// --format json 付きで実行し、JSON パース結果を返す
pub fn todos_json(data_dir: &Path, args: &[&str]) -> serde_json::Value {
    let output = todos_cmd(data_dir)
        .args(args)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

/// stdin 付きで実行し、JSON パース結果を返す
pub fn todos_json_stdin(data_dir: &Path, args: &[&str], stdin: &str) -> serde_json::Value {
    let output = todos_cmd(data_dir)
        .args(args)
        .arg("--format")
        .arg("json")
        .write_stdin(stdin)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

/// --format json --yes 付きで実行し、JSON パース結果を返す
pub fn todos_json_yes(data_dir: &Path, args: &[&str]) -> serde_json::Value {
    let output = todos_cmd(data_dir)
        .args(args)
        .arg("--format")
        .arg("json")
        .arg("--yes")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}
