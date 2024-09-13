use anyhow::Result;
use std::{env::temp_dir, io::Write, path::PathBuf};

pub fn ensure_roslyn_is_installed() -> Result<PathBuf> {
    let mut version_dir = home::home_dir().expect("Unable to find home directory");
    version_dir.push(".roslyn");
    version_dir.push("server");
    version_dir.push(VERSION);
    fs_extra::dir::create_all(&version_dir, false)?;

    let mut dll_path = version_dir.clone();
    dll_path.push("Microsoft.CodeAnalysis.LanguageServer.dll");

    if std::path::Path::new(&dll_path).exists() {
        return Ok(dll_path);
    }

    let mut temp_dir = temp_dir();
    temp_dir.push("roslyn");
    fs_extra::dir::create(&temp_dir, true)?;

    let mut nuget_config_file = std::fs::File::create(temp_dir.join("NuGet.config"))?;
    nuget_config_file.write_all(NUGET.as_bytes())?;

    let mut csproj_file = std::fs::File::create(temp_dir.join("ServerDownload.csproj")).unwrap();
    csproj_file.write_all(CSPROJ.as_bytes())?;

    std::thread::sleep(std::time::Duration::from_secs(1));

    std::process::Command::new("dotnet")
        .arg("add")
        .arg("package")
        .arg("Microsoft.CodeAnalysis.LanguageServer.neutral")
        .arg("-v")
        .arg(VERSION)
        .current_dir(&temp_dir)
        .output()?;

    temp_dir.push("out");
    temp_dir.push("microsoft.codeanalysis.languageserver.neutral");
    temp_dir.push(VERSION);
    temp_dir.push("content");
    temp_dir.push("LanguageServer");
    temp_dir.push("neutral");

    let copy_options = fs_extra::dir::CopyOptions::default()
        .overwrite(true)
        .content_only(true);

    fs_extra::dir::move_dir(&temp_dir, &version_dir, &copy_options)?;
    fs_extra::dir::remove(temp_dir)?;

    Ok(dll_path)
}

pub const VERSION: &str = "4.12.0-3.24461.2";

const NUGET: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<configuration>
  <packageSources>
    <clear />

    <add key=\"vs-impl\" value=\"https://pkgs.dev.azure.com/azure-public/vside/_packaging/vs-impl/nuget/v3/index.json\" />

  </packageSources>
</configuration>
    ";

const CSPROJ: &str = "<Project Sdk=\"Microsoft.NET.Sdk\">
    <PropertyGroup>
        <RestorePackagesPath>out</RestorePackagesPath>
        <TargetFramework>net8.0</TargetFramework>
        <DisableImplicitNuGetFallbackFolder>true</DisableImplicitNuGetFallbackFolder>
        <AutomaticallyUseReferenceAssemblyPackages>false</AutomaticallyUseReferenceAssemblyPackages>
    </PropertyGroup>
</Project>
";
