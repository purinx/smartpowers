//! The web UI in `assets/board`, embedded into the binary.

pub struct Asset {
    pub path: &'static str,
    pub content_type: &'static str,
    pub body: &'static str,
}

const HTML: &str = "text/html; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";

/// Embed `assets/board<path>` and serve it at `<path>`.
macro_rules! asset {
    ($path:literal, $content_type:expr) => {
        Asset {
            path: $path,
            content_type: $content_type,
            body: include_str!(concat!("../../../assets/board", $path)),
        }
    };
}

/// Every file under `assets/board` must be listed here;
/// the `embeds_every_asset_file` test enforces it.
const ASSETS: &[Asset] = &[
    asset!("/index.html", HTML),
    asset!("/app.css", CSS),
    asset!("/js/main.js", JS),
    asset!("/js/api.js", JS),
    asset!("/js/dom.js", JS),
    asset!("/js/live.js", JS),
    asset!("/js/markdown.js", JS),
    asset!("/js/router.js", JS),
    asset!("/js/state.js", JS),
    asset!("/js/status.js", JS),
    asset!("/js/views/activity.js", JS),
    asset!("/js/views/board.js", JS),
    asset!("/js/views/composer.js", JS),
    asset!("/js/views/content.js", JS),
    asset!("/js/views/flow.js", JS),
    asset!("/js/views/header.js", JS),
    asset!("/js/views/list.js", JS),
    asset!("/js/views/panel.js", JS),
    asset!("/js/views/panel-resize.js", JS),
    asset!("/js/views/sidebar.js", JS),
    asset!("/js/views/task-parts.js", JS),
];

/// Exact-match lookup, so paths like `/../x` can never escape the table.
pub fn find(path: &str) -> Option<&'static Asset> {
    let path = if path == "/" { "/index.html" } else { path };
    ASSETS.iter().find(|asset| asset.path == path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::files_recursive;
    use std::path::Path;

    #[test]
    fn embeds_every_asset_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/board");
        let mut on_disk: Vec<String> = files_recursive(&root)
            .iter()
            .filter(|path| path.file_name().is_some_and(|name| name != ".DS_Store"))
            .map(|path| {
                let relative = path.strip_prefix(&root).unwrap().to_string_lossy();
                format!("/{}", relative.replace('\\', "/"))
            })
            .collect();
        let mut embedded: Vec<String> = ASSETS.iter().map(|asset| asset.path.to_string()).collect();
        on_disk.sort();
        embedded.sort();
        assert_eq!(on_disk, embedded, "update ASSETS to match assets/board");
    }

    #[test]
    fn finds_assets_by_exact_path() {
        assert_eq!(find("/").map(|asset| asset.content_type), Some(HTML));
        assert_eq!(
            find("/js/main.js").map(|asset| asset.content_type),
            Some(JS)
        );
        assert!(find("/../Cargo.toml").is_none());
        assert!(find("/js/").is_none());
    }
}
