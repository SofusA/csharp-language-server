use std::process::Stdio;

use anyhow::{bail, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStdout, Command},
};

use crate::{
    download_roslyn::ensure_roslyn_is_installed,
    pipe_stream::{Pipe, PipeStream},
};

#[derive(Serialize, Deserialize)]
struct RoslynResponse {
    #[serde(rename = "pipeName")]
    pipe_name: String,
}

async fn parse_roslyn_response(reader: BufReader<ChildStdout>) -> Result<RoslynResponse> {
    let first_line = reader
        .lines()
        .next_line()
        .await?
        .context("No lines to read")?;

    match serde_json::from_str::<RoslynResponse>(&first_line) {
        Ok(res) => Ok(res),
        Err(_) => bail!("{first_line}"),
    }
}

pub async fn start_roslyn(version: &str, remove_old_server_versions: bool) -> Box<dyn PipeStream> {
    let cache_dir = ProjectDirs::from("com", "github", "roslyn-language-server")
        .expect("Unable to find cache directory")
        .cache_dir()
        .to_path_buf();

    let log_dir = cache_dir.join("log");

    let mut process: Child;

    let roslyn_dll = ensure_roslyn_is_installed(version, remove_old_server_versions, &cache_dir)
        .await
        .expect("Unable to install Roslyn");

    process = Command::new("dotnet")
        .arg(roslyn_dll)
        .arg("--logLevel=Information")
        .arg("--extensionLogDirectory")
        .arg(log_dir)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let reader = BufReader::new(process.stdout.take().expect("Failed to capture stdout"));

    let roslyn_response = parse_roslyn_response(reader)
        .await
        .expect("Unable to parse response from server");

    Pipe::connect(&roslyn_response.pipe_name)
        .await
        .expect("Unable to connect to server stream")
}
