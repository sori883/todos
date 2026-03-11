mod helpers;

use assert_cmd::Command;

#[test]
fn binary_exists_and_runs() {
    Command::cargo_bin("todos")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}
