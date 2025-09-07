use std::{path::PathBuf, str::FromStr};

use clap::Parser;
use futures::future::try_join;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};

use csharp_language_server::{
    path::create_open_notification,
    server::{download_server, start_server},
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

    /// Disable sending any "open" commands.
    #[arg(short, long, default_value_t = false)]
    no_open: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let version = SERVER_VERSION;
    let directory_path = args.directory.map(|dir| PathBuf::from_str(&dir).unwrap());

    if args.download {
        println!("Downloading language server");

        download_server(version, args.remove_old_server_versions, directory_path).await;

        println!("Done!");
        return;
    }

    let (mut server_stdin, server_stdout) =
        start_server(version, args.remove_old_server_versions, directory_path).await;

    let stdin = io::stdin();

    let stream_to_stdout = async {
        let mut reader = BufReader::new(server_stdout);
        let mut stdout = io::stdout();

        io::copy(&mut reader, &mut stdout).await
    };

    let stdin_to_stream = async {
        let mut stdin = BufReader::new(stdin);
        if !args.no_open {
            loop {
                let mut buffer = vec![0; 6000];
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

                let notification = String::from_utf8(buffer[..bytes_read].to_vec())
                    .expect("Unable to convert buffer to string");

                if notification.contains("initialize") {
                    let open_solution_notification = create_open_notification(
                        &notification,
                        args.solution_path,
                        args.project_paths,
                    );

                    server_stdin
                        .write_all(open_solution_notification.as_bytes())
                        .await
                        .expect("Unable to send open solution notification to server");

                    break;
                }
            }
        }
        io::copy(&mut stdin, &mut server_stdin).await
    };

    try_join(stdin_to_stream, stream_to_stdout)
        .await
        .expect("Will never finish");
}
