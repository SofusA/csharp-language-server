use anyhow::Result;
use std::{
    fs::File,
    io::Cursor,
    path::{Path, PathBuf},
};
use zip::ZipArchive;

pub async fn ensure_roslyn_is_installed(
    version: &str,
    remove_old_server_versions: bool,
    cache_dir: &Path,
) -> Result<PathBuf> {
    let roslyn_server_dir = cache_dir.join("server");
    let dll_version_dir = roslyn_server_dir.join(version);
    let dll_path = dll_version_dir.join("Microsoft.CodeAnalysis.LanguageServer.dll");

    // return if language server is already extracted
    if std::path::Path::new(&dll_path).exists() {
        return Ok(dll_path);
    }

    fs_extra::dir::create_all(&roslyn_server_dir, remove_old_server_versions)?;
    fs_extra::dir::create_all(&dll_version_dir, true)?;

    let language_server_zip = include_bytes!("../language-server.zip");

    // extract language server
    let reader = Cursor::new(language_server_zip);
    let mut archive = ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = dll_version_dir.join(file.name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(dll_path)
}
