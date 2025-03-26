use assert_cmd::cargo::CommandCargoExt;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

#[test]
fn first_line_is_jsonrpc() {
    #[allow(clippy::zombie_processes)]
    let mut cmd = Command::cargo_bin("csharp-language-server")
        .unwrap()
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start process");

    let stdout = cmd.stdout.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let first_line = lines
        .next()
        .expect("No output received")
        .expect("Failed to read line");

    cmd.kill().unwrap();

    // language server responds with a jsonrpc message
    assert!(first_line.contains("Content-Length"));
}
