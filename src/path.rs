use serde_json::Value;
use std::path::PathBuf;
use url::Url;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Path(PathBuf);

#[macro_export]
macro_rules! find_extension {
    ($root_path:expr, $($extension:expr),+) => {
        rust_search::SearchBuilder::default()
            .location(Into::<PathBuf>::into($root_path))
            $(.ext($extension))+
            .build()
    };
}

impl Path {
    pub fn try_from_uri(uri: &str) -> Option<Self> {
        uri.parse::<Url>()
            .expect("Couldn't parse path URI")
            .to_file_path()
            .ok()
            .map(Self)
    }

    pub fn from_path_str(path: &str) -> Self {
        Self(PathBuf::from(path))
    }

    pub fn to_uri_string(&self) -> String {
        Url::from_file_path(&self.0)
            .unwrap_or_else(|_| panic!("Couldn't turn {:?} into Uri", self.0))
            .to_string()
    }

    pub fn join(&self, part: &str) -> Self {
        Path(self.0.join(part.trim()))
    }
}

impl From<Path> for PathBuf {
    fn from(value: Path) -> Self {
        value.0
    }
}

pub fn parse_root_path(notification: &str) -> Result<Path> {
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
                    .map(Path::from_path_str)
            },
            Path::try_from_uri,
        )
        .context("Root URI/path was not given by the client")?;

    Ok(root_path)
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
                let expected = $expected.replace('/', &std::path::MAIN_SEPARATOR.to_string());
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
