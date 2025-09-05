use anyhow::{Result, bail};
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

    let server_dll = ensure_server_is_installed(version, remove_old_server_versions, &dir)
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

async fn ensure_server_is_installed(
    version: &str,
    remove_old_server_versions: bool,
    server_dir: &Path,
) -> Result<PathBuf> {
    let dll_version_dir = server_dir.join(version);
    let dll_path = dll_version_dir.join("Microsoft.CodeAnalysis.LanguageServer.dll");

    if std::path::Path::new(&dll_path).exists() {
        return Ok(dll_path);
    }

    let dotnet_sdk_output = match Command::new("dotnet").arg("--list-sdks").output().await {
        Ok(output) => String::from_utf8(output.stdout)?,
        Err(_) => bail!("Unable to get dotnet sdk version. Is dotnet installed?"),
    };

    let dotnet_sdk_version = match dotnet_sdk_output
        .lines()
        .filter_map(|line| line.split('.').next())
        .next_back()
    {
        Some(version) => version,
        None => bail!("Unable to get dotnet sdk version. No sdk installations found"),
    };

    let dotnet_sdk_version_string = match dotnet_sdk_version {
        "5" => "net5.0",
        "6" => "net6.0",
        "7" => "net7.0",
        "8" => "net8.0",
        "9" => "net9.0",
        "10" => "net10.0",
        _ => bail!("Unsupported dotnet sdk: {}", dotnet_sdk_version),
    };

    fs_extra::dir::create_all(server_dir, remove_old_server_versions)?;
    fs_extra::dir::create_all(&dll_version_dir, true)?;

    let temp_build_root = temp_dir().join("csharp-language-server");
    fs_extra::dir::create(&temp_build_root, true)?;

    create_csharp_project(&temp_build_root, dotnet_sdk_version_string)?;

    Command::new("dotnet")
        .arg("add")
        .arg("package")
        .arg("Microsoft.CodeAnalysis.LanguageServer.neutral")
        .arg("-v")
        .arg(version)
        .current_dir(fs::canonicalize(temp_build_root.clone())?)
        .output()
        .await?;

    let temp_build_dir = temp_build_root
        .join("out")
        .join("microsoft.codeanalysis.languageserver.neutral")
        .join(version)
        .join("content")
        .join("LanguageServer")
        .join("neutral");

    let copy_options = fs_extra::dir::CopyOptions::default()
        .overwrite(true)
        .content_only(true);

    fs_extra::dir::move_dir(&temp_build_dir, &dll_version_dir, &copy_options)?;
    fs_extra::dir::remove(temp_build_dir)?;

    Ok(dll_path)
}

fn create_csharp_project(temp_dir: &Path, dotnet_sdk_version_string: &str) -> Result<()> {
    let mut nuget_config_file = std::fs::File::create(temp_dir.join("NuGet.config"))?;
    nuget_config_file.write_all(NUGET.as_bytes())?;

    let mut csproj_file = std::fs::File::create(temp_dir.join("ServerDownload.csproj")).unwrap();
    csproj_file.write_all(csproj_string(dotnet_sdk_version_string).as_bytes())?;

    Ok(())
}

const NUGET: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<configuration>
  <packageSources>
    <clear />

    <add key=\"vs-impl\" value=\"https://pkgs.dev.azure.com/azure-public/vside/_packaging/vs-impl/nuget/v3/index.json\" />

  </packageSources>
</configuration>
    ";

fn csproj_string(dotnet_sdk_version_string: &str) -> String {
    format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">
            <PropertyGroup>
                <RestorePackagesPath>out</RestorePackagesPath>
                <TargetFramework>{dotnet_sdk_version_string}</TargetFramework>
                <DisableImplicitNuGetFallbackFolder>true</DisableImplicitNuGetFallbackFolder>
                <AutomaticallyUseReferenceAssemblyPackages>false</AutomaticallyUseReferenceAssemblyPackages>
            </PropertyGroup>
         </Project>"
    )
}
