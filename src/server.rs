use std::process::Stdio;

use anyhow::{bail, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStdout, Command},
};

use crate::{
    download_server::ensure_server_is_installed,
    pipe_stream::{Pipe, PipeStream},
};

#[derive(Serialize, Deserialize)]
struct ServerResponse {
    #[serde(rename = "pipeName")]
    pipe_name: String,
}

async fn parse_server_response(reader: BufReader<ChildStdout>) -> Result<ServerResponse> {
    let first_line = reader
        .lines()
        .next_line()
        .await?
        .context("No lines to read")?;

    match serde_json::from_str::<ServerResponse>(&first_line) {
        Ok(res) => Ok(res),
        Err(_) => bail!("{first_line}"),
    }
}

pub async fn start_server(version: &str, remove_old_server_versions: bool) -> Box<dyn PipeStream> {
    let cache_dir = ProjectDirs::from("com", "github", "csharp-language-server")
        .expect("Unable to find cache directory")
        .cache_dir()
        .to_path_buf();

    let log_dir = cache_dir.join("log");

    let mut process: Child;

    let server_dll = ensure_server_is_installed(version, remove_old_server_versions, &cache_dir)
        .await
        .expect("Unable to install server");

    process = Command::new("dotnet")
        .arg(server_dll)
        .arg("--logLevel=Information")
        .arg("--extensionLogDirectory")
        .arg(log_dir)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let reader = BufReader::new(process.stdout.take().expect("Failed to capture stdout"));

    let server_response = parse_server_response(reader)
        .await
        .expect("Unable to parse response from server");

    Pipe::connect(&server_response.pipe_name)
        .await
        .expect("Unable to connect to server stream")
}
