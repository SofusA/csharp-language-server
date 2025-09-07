use anyhow::Result;
use directories::ProjectDirs;
use std::process::Stdio;
use std::{
    env::temp_dir,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tokio::process::Command;

pub async fn start_server(
    version: &str,
    remove_old_server_versions: bool,
    override_directory: Option<PathBuf>,
) -> (tokio::process::ChildStdin, tokio::process::ChildStdout) {
    let dir = override_directory.unwrap_or(cache_dir());
    let log_dir = cache_dir().join("log");

    let server = ensure_server_is_installed(version, remove_old_server_versions, &dir)
        .await
        .expect("Unable to install server");

    let mut command = match server {
        ServerPath::Exe(path) => Command::new(path),
        ServerPath::Dll(path) => {
            let mut cmd = Command::new("dotnet");
            cmd.arg("exec");
            cmd.arg(path);
            cmd
        }
    };

    let command = command
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

pub async fn download_server(
    version: &str,
    remove_old_server_versions: bool,
    override_directory: Option<PathBuf>,
) {
    let dir = override_directory.unwrap_or(cache_dir());

    ensure_server_is_installed(version, remove_old_server_versions, &dir)
        .await
        .expect("Unable to install server");
}

fn cache_dir() -> PathBuf {
    let cache_dir = ProjectDirs::from("com", "github", "csharp-language-server")
        .expect("Unable to find cache directory")
        .cache_dir()
        .to_path_buf();

    cache_dir.join("server")
}

enum ServerPath {
    Exe(PathBuf),
    Dll(PathBuf),
}

async fn ensure_server_is_installed(
    version: &str,
    remove_old_server_versions: bool,
    server_root_dir: &Path,
) -> Result<ServerPath> {
    let server_dir = server_root_dir.join(version);

    let rid = current_rid();
    if std::path::Path::new(&server_dir.join(rid)).exists() {
        return get_server_path(&server_dir, rid);
    }

    fs_extra::dir::create_all(server_root_dir, remove_old_server_versions)?;
    fs_extra::dir::create_all(&server_dir, true)?;

    let temp_build_root = temp_dir().join("csharp-language-server");
    fs_extra::dir::create(&temp_build_root, true)?;

    create_csharp_project(&temp_build_root)?;

    let res = Command::new("dotnet")
        .arg("restore")
        .arg(format!(
            "-p:LanguageServerPackage=Microsoft.CodeAnalysis.LanguageServer.{rid}"
        ))
        .arg(format!("-p:LanguageServerVersion={version}"))
        .current_dir(fs::canonicalize(temp_build_root.clone())?)
        .output()
        .await?;

    anyhow::ensure!(
        res.status.success(),
        "dotnet restore failed with exit code: {:?}\nstdout: {}\nstderr: {}",
        res.status.code(),
        String::from_utf8_lossy(&res.stdout),
        String::from_utf8_lossy(&res.stderr)
    );

    let temp_build_dir = temp_build_root
        .join("out")
        .join(format!("microsoft.codeanalysis.languageserver.{rid}"))
        .join(version)
        .join("content")
        .join("LanguageServer");

    let copy_options = fs_extra::dir::CopyOptions::default()
        .overwrite(true)
        .content_only(true);

    fs_extra::dir::move_dir(&temp_build_dir, &server_dir, &copy_options)?;
    fs_extra::dir::remove(temp_build_dir)?;

    get_server_path(&server_dir, rid)
}

fn create_csharp_project(temp_build_root: &Path) -> Result<()> {
    let mut csproj_file = std::fs::File::create(temp_build_root.join("ServerDownload.csproj"))?;
    csproj_file.write_all(CSPROJ.as_bytes())?;
    Ok(())
}

fn get_server_path(server_dir: &Path, rid: &str) -> Result<ServerPath> {
    let exe_dir = server_dir.join(rid);
    Ok(if rid == "neutral" {
        ServerPath::Dll(exe_dir.join("Microsoft.CodeAnalysis.LanguageServer.dll"))
    } else if rid.starts_with("win-") {
        ServerPath::Exe(exe_dir.join("Microsoft.CodeAnalysis.LanguageServer.exe"))
    } else {
        ServerPath::Exe(exe_dir.join("Microsoft.CodeAnalysis.LanguageServer"))
    })
}

const CSPROJ: &str = r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <RestoreSources>https://pkgs.dev.azure.com/azure-public/vside/_packaging/vs-impl/nuget/v3/index.json</RestoreSources>
        <RestorePackagesPath>out</RestorePackagesPath>
        <TargetFramework>netstandard2.0</TargetFramework>
        <DisableImplicitNuGetFallbackFolder>true</DisableImplicitNuGetFallbackFolder>
        <DisableImplicitFrameworkReferences>true</DisableImplicitFrameworkReferences>
    </PropertyGroup>

    <ItemGroup>
        <PackageDownload Include="$(LanguageServerPackage)" Version="[$(LanguageServerVersion)]" />
    </ItemGroup>
</Project>"#;

#[allow(unreachable_code)]
const fn current_rid() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "win-x64";

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    return "win-arm64";

    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
    return "linux-musl-x64";

    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    return "linux-x64";

    #[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "musl"))]
    return "linux-musl-arm64";

    #[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "gnu"))]
    return "linux-arm64";

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "osx-x64";

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "osx-arm64";

    "neutral"
}
