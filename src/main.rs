use std::{path::PathBuf, str::FromStr};

use anyhow::Result;
use clap::Parser;
use futures::future::try_join;
use serde_json::{Value, json};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};

use csharp_language_server::{
    find_extension,
    notification::{
        Notification, Params, ProjectParams, SolutionParams, add_content_length_header,
    },
    path::{Path, parse_root_path},
    server::start_server,
    server_version::SERVER_VERSION,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Remove old versions of Microsoft.CodeAnalysis.LanguageServer
    #[arg(short, long, default_value_t = true)]
    remove_old_server_versions: bool,

    /// Download Microsoft.CodeAnalysis.LanguageServer and exit
    #[arg(long, default_value_t = false)]
    download: bool,

    /// Override directory to download and execute Microsoft.CodeAnalysis.LanguageServer
    #[arg(short, long)]
    directory: Option<String>,

    /// Override solution (.sln) path. Absolute path
    #[arg(short, long)]
    solution_path: Option<String>,

    /// Override project(s) (.csproj) path(s). Absolute path. Solution path takes precedence
    #[arg(short, long)]
    project_paths: Option<Vec<String>>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let version = SERVER_VERSION;
    let directory_path = args.directory.map(|dir| PathBuf::from_str(&dir).unwrap());

    if args.download {
        println!("Downloading language server");

        csharp_language_server::server::download_server(
            version,
            args.remove_old_server_versions,
            directory_path,
        )
        .await;

        println!("Done!");
        return;
    }

    let (mut server_stdin, server_stdout) =
        start_server(version, args.remove_old_server_versions, directory_path).await;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let stream_to_stdout = async {
        let mut reader = BufReader::new(server_stdout);
        let mut buffer = vec![0; 3048];
        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .await
                .expect("Unable to read incoming server notification");

            if bytes_read == 0 {
                break; // EOF reached
            }

            if let Ok(notification) = str::from_utf8(&buffer[..bytes_read]) {
                if notification.contains("capabilities") {
                    let patched_result_notification = force_pull_diagnostics_hack(notification)?;

                    stdout
                        .write_all(patched_result_notification.as_bytes())
                        .await?;

                    break;
                }

                stdout
                    .write_all(&buffer[..bytes_read])
                    .await
                    .expect("Unable to forward client notification to server");
                buffer.clear();
            }
        }
        drop(buffer);
        io::copy(&mut reader, &mut stdout).await
    };

    let stdin_to_stream = async {
        let mut stdin = BufReader::new(stdin);
        let mut buffer = vec![0; 6000];
        loop {
            let bytes_read = stdin
                .read(&mut buffer)
                .await
                .expect("Unable to read incoming client notification");

            if bytes_read == 0 {
                break; // EOF reached
            }

            server_stdin
                .write_all(&buffer[..bytes_read])
                .await
                .expect("Unable to forward client notification to server");

            if let Ok(notification) = str::from_utf8(&buffer[..bytes_read]) {
                if notification.contains("initialize") {
                    server_stdin
                        .write_all(
                            create_open_notification(
                                notification,
                                &args.solution_path,
                                &args.project_paths,
                            )
                            .as_bytes(),
                        )
                        .await
                        .expect("Unable to send open projects notification to server");
                    break;
                }
            }
            buffer.clear();
        }
        drop(buffer);
        io::copy(&mut stdin, &mut server_stdin).await
    };

    try_join(stdin_to_stream, stream_to_stdout)
        .await
        .expect("Will never finish");
}

fn create_open_notification(
    init_notification: &str,
    solution_override: &Option<String>,
    projects_override: &Option<Vec<String>>,
) -> String {
    let mut root_path =
        parse_root_path(init_notification).expect("Root path not part of initialize notification");

    let open_solution_notification = open_solution_notification(&mut root_path, solution_override);

    if let Some(open_solution_notification) = open_solution_notification {
        return open_solution_notification;
    }

    open_projects_notification(&root_path, projects_override)
}

fn open_solution_notification(
    root_path: &mut Path,
    override_path: &Option<String>,
) -> Option<String> {
    let solution_path = match override_path {
        Some(p) => root_path.join(p),
        None => {
            let mut solution_files = find_extension!(root_path.clone(), "sln", "slnx");
            root_path.join(&solution_files.next()?)
        }
    };

    Some(
        Notification {
            jsonrpc: "2.0".to_string(),
            method: "solution/open".to_string(),
            params: Params::Solution(SolutionParams {
                solution: solution_path.to_uri_string(),
            }),
        }
        .serialize(),
    )
}

fn open_projects_notification(root_path: &Path, override_paths: &Option<Vec<String>>) -> String {
    let file_paths = match override_paths {
        Some(p) => p,
        None => &find_extension!(root_path.clone(), "csproj").collect(),
    };

    let uris: Vec<String> = file_paths
        .iter()
        .map(|p| root_path.join(p).to_uri_string())
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
