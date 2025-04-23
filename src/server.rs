use directories::ProjectDirs;
use std::process::Stdio;
use tokio::process::Command;

use crate::download_server::ensure_server_is_installed;

pub async fn start_server(
    version: &str,
    remove_old_server_versions: bool,
) -> (tokio::process::ChildStdin, tokio::process::ChildStdout) {
    let cache_dir = ProjectDirs::from("com", "github", "csharp-language-server")
        .expect("Unable to find cache directory")
        .cache_dir()
        .to_path_buf();

    let log_dir = cache_dir.join("log");

    let server_dll = ensure_server_is_installed(version, remove_old_server_versions, &cache_dir)
        .await
        .expect("Unable to install server");

    let command = Command::new("dotnet")
        .arg(server_dll)
        .arg("--logLevel=Information")
        .arg("--extensionLogDirectory")
        .arg(log_dir)
        .arg("--stdio")
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    (command.stdin.unwrap(), command.stdout.unwrap())
}
