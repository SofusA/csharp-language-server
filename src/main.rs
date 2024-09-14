use ::futures::future::try_join;
use anyhow::{Context, Result};
use clap::Parser;
use rust_search::SearchBuilder;
use serde_json::{json, Value};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};

use roslyn_language_server::{
    notification::{
        add_content_length_header, Notification, Params, ProjectParams, SolutionParams,
    },
    roslyn::start_roslyn,
    server_version::SERVER_VERSION,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Provide path for Microsoft.CodeAnalysis.LanguageServer.
    /// Otherwise this will be downloaded to ~/.roslyn
    #[arg(short, long)]
    server_path: Option<String>,

    /// Remove old versions of Microsoft.CodeAnalysis.LanguageServer
    #[arg(short, long, default_value_t = true)]
    remove_old_server_versions: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let version = SERVER_VERSION;

    let pipe = start_roslyn(args.server_path, version, args.remove_old_server_versions).await;

    let (reader, mut writer) = tokio::io::split(pipe);

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let stream_to_stdout = async {
        let mut reader = BufReader::new(reader);
        loop {
            let mut buffer = vec![0; 3048];
            let bytes_read = reader
                .read(&mut buffer)
                .await
                .expect("Unable to read incoming server notification");

            if bytes_read == 0 {
                break; // EOF reached
            }

            let notification = String::from_utf8(buffer[..bytes_read].to_vec())
                .expect("Unable to convert buffer to string");

            if notification.contains("capabilities") {
                let patched_result_notification = force_pull_diagnostics_hack(&notification)?;

                stdout
                    .write_all(patched_result_notification.as_bytes())
                    .await?;

                break;
            }

            stdout
                .write_all(&buffer[..bytes_read])
                .await
                .expect("Unable to forward client notification to server");
        }

        io::copy(&mut reader, &mut stdout).await
    };

    let stdin_to_stream = async {
        let mut stdin = BufReader::new(stdin);
        loop {
            let mut buffer = vec![0; 3048];
            let bytes_read = stdin
                .read(&mut buffer)
                .await
                .expect("Unable to read incoming client notification");

            if bytes_read == 0 {
                break; // EOF reached
            }

            writer
                .write_all(&buffer[..bytes_read])
                .await
                .expect("Unable to forward client notification to server");

            let notification = String::from_utf8(buffer[..bytes_read].to_vec())
                .expect("Unable to convert buffer to string");

            if notification.contains("initialize") {
                let root_path = parse_root_path(&notification)
                    .expect("Root path not part of initialize notification");

                let solution_files = find_extension(&root_path, "sln");
                let solution_to_open = solution_files.first().map(|found| found.to_owned());

                if let Some(solution_to_open) = solution_to_open {
                    let open_solution_notification =
                        create_open_solution_notification(&solution_to_open);

                    writer
                        .write_all(open_solution_notification.as_bytes())
                        .await
                        .expect("Unable to send open solution notification to server");

                    break;
                }

                let project_files = find_extension(&root_path, "csproj");
                let open_projects_notification = create_open_projects_notification(project_files);

                writer
                    .write_all(open_projects_notification.as_bytes())
                    .await
                    .expect("Unable to send open projects notification to server");

                break;
            }
        }
        io::copy(&mut stdin, &mut writer).await
    };

    try_join(stdin_to_stream, stream_to_stdout)
        .await
        .expect("Will never finish");
}

fn parse_root_path(notification: &str) -> Result<String> {
    let json_start = notification
        .find('{')
        .context("Notification was not json")?;

    let parsed_notification: Value = serde_json::from_str(&notification[json_start..])?;

    let root_path = parsed_notification["params"]["rootPath"]
        .as_str()
        .context("Root path")?;

    Ok(root_path.to_string())
}

fn find_extension(root_path: &str, extension: &str) -> Vec<String> {
    SearchBuilder::default()
        .location(root_path)
        .ext(extension)
        .build()
        .collect()
}

fn create_open_solution_notification(file_path: &str) -> String {
    let notification = Notification {
        jsonrpc: "2.0".to_string(),
        method: "solution/open".to_string(),
        params: Params::Solution(SolutionParams {
            solution: path_to_uri(file_path),
        }),
    };

    notification.serialize()
}

fn path_to_uri(file_path: &str) -> String {
    format!("file://{file_path}")
}

fn create_open_projects_notification(file_paths: Vec<String>) -> String {
    let uris: Vec<String> = file_paths
        .iter()
        .map(|file_path| path_to_uri(file_path))
        .collect();

    let notification = Notification {
        jsonrpc: "2.0".to_string(),
        method: "project/open".to_string(),
        params: Params::Project(ProjectParams { projects: uris }),
    };

    notification.serialize()
}

fn force_pull_diagnostics_hack(notification: &str) -> Result<String, std::io::Error> {
    let json_start = notification.find('{').ok_or(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "No JSON start found",
    ))?;
    let mut parsed_notification: Value = serde_json::from_str(&notification[json_start..])?;

    let diagnostic_provider = json!({
        "interFileDependencies": true,
        "workDoneProgress": true,
        "workspaceDiagnostics": true
    });

    parsed_notification["result"]["capabilities"]["diagnosticProvider"] = diagnostic_provider;

    Ok(add_content_length_header(&parsed_notification.to_string()))
}
