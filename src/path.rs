use serde_json::Value;
use std::{ffi::OsStr, path::PathBuf};
use url::Url;
use walkdir::{DirEntry, WalkDir};

use anyhow::{Context, Result};

use crate::notification::{Notification, Params, ProjectParams, SolutionParams};

pub fn create_open_notification(
    notification: &str,
    solution_override: Option<String>,
    projects_override: Option<Vec<String>>,
) -> String {
    let root_path =
        parse_root_path(notification).expect("Root path not part of initialize notification");

    let open_solution_notification = open_solution_notification(&root_path, solution_override);

    if let Some(open_solution_notification) = open_solution_notification {
        return open_solution_notification;
    }

    open_projects_notification(&root_path, projects_override)
}

fn open_solution_notification(root_path: &Path, override_path: Option<String>) -> Option<String> {
    let solution_path = match override_path {
        Some(p) => root_path.join(&p),
        None => find_extension(root_path, &vec![OsStr::new("sln"), OsStr::new("slnx")]).next()?,
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

fn open_projects_notification(root_path: &Path, override_paths: Option<Vec<String>>) -> String {
    let file_paths = match override_paths {
        Some(p) => p,
        None => find_extension(root_path, &vec![OsStr::new("csproj")])
            .map(|p| p.to_uri_string())
            .collect(),
    };

    let notification = Notification {
        jsonrpc: "2.0".to_string(),
        method: "project/open".to_string(),
        params: Params::Project(ProjectParams {
            projects: file_paths,
        }),
    };

    notification.serialize()
}

#[derive(Debug, Clone)]
struct Path(PathBuf);

impl Path {
    fn try_from_uri(uri: &str) -> Option<Self> {
        uri.parse::<Url>()
            .expect("Couldn't parse path URI")
            .to_file_path()
            .ok()
            .map(Self)
    }

    fn to_uri_string(&self) -> String {
        Url::from_file_path(&self.0)
            .unwrap_or_else(|_| panic!("Couldn't turn {:?} into Uri", self.0))
            .to_string()
    }

    fn join(&self, part: &str) -> Self {
        Path(self.0.join(part.trim()))
    }
}

impl From<&str> for Path {
    fn from(path: &str) -> Self {
        Self(PathBuf::from(path))
    }
}

impl From<&DirEntry> for Path {
    fn from(value: &DirEntry) -> Self {
        value.path().to_str().unwrap().into()
    }
}

fn parse_root_path(notification: &str) -> Result<Path> {
    let json_start = notification
        .find('{')
        .context("Notification was not json")?;

    let parsed_notification: Value = serde_json::from_str(&notification[json_start..])?;

    let root_path = parsed_notification["params"]["rootUri"]
        .as_str()
        .map_or_else(
            || {
                parsed_notification["params"]["rootPath"]
                    .as_str()
                    .map(|p| p.into())
            },
            Path::try_from_uri,
        )
        .context("Root URI/path was not given by the client")?;

    Ok(root_path)
}

fn path_for_file_with_extension(dir: &DirEntry, ext: &Vec<&'static OsStr>) -> Option<Path> {
    if dir.path().is_file() && dir.path().extension().is_some_and(|e| ext.contains(&e)) {
        return Some(dir.into());
    }
    None
}

fn find_extension(root_path: &Path, ext: &Vec<&'static OsStr>) -> impl Iterator<Item = Path> {
    WalkDir::new(&root_path.0)
        .into_iter()
        .filter_map(|d| d.ok())
        .filter_map(|d| path_for_file_with_extension(&d, ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! from_uri_test {
    ([$target_family:literal], $($test_name:ident: ($input:expr, $expected:expr),)*) => {
        $(
            #[cfg(target_family = $target_family)]
            #[test]
            fn $test_name() {
                let from_uri =
                    Path::try_from_uri($input);
                    let expected = $expected.replace('/', std::path::MAIN_SEPARATOR_STR);
                assert!(from_uri.as_ref().is_some_and(|p| p.0.to_str().is_some_and(|s| s.eq(&expected))),
                    "try_from_uri for '{}' was '{:?}' and not '{}'", $input, &from_uri, expected
                )
            }
        )*
    }
}

    from_uri_test! { ["windows"],
        rooted_windows: ("file:///C:/_Foo/bar/baz","C:/_Foo/bar/baz"),
        with_host_windows: ("file://localhost/C:/_Foo/bar/baz","C:/_Foo/bar/baz"),
        network_path_windows: ("file://_Foo/bar/baz","//_foo/bar/baz"),
        with_parent_windows: ("file://_Foo/bar/../baz","//_foo/baz"),
    }

    from_uri_test! { ["unix"],
        rooted_unix: ("file:///var/_Foo/bar/baz","/var/_Foo/bar/baz"),
        with_host_unix: ("file://localhost/_Foo/bar/baz","/_Foo/bar/baz"),
        with_parent_unix: ("file:///var/_Foo/bar/../baz","/var/_Foo/baz"),
    }
}
