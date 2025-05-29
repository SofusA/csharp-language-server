use anyhow::Result;
use std::{
    fs::File,
    io::Cursor,
    path::{Path, PathBuf},
};
use zip::ZipArchive;

pub async fn ensure_server_is_installed(
    version: &str,
    _remove_old_server_versions: bool,
    cache_dir: &Path,
) -> Result<PathBuf> {
    let server_dir = cache_dir.join("server");

    let dll_version_dir = server_dir.join(version);

    let dll_path = dll_version_dir.join("Microsoft.CodeAnalysis.LanguageServer.dll");

    if std::path::Path::new(&dll_path).exists() {
        return Ok(dll_path);
    }

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
