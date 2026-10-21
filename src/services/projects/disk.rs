//! How much room a folder takes, for the actions that free it.
use std::{fs, path::Path};

/// The bytes in `path` and everything under it. A file it cannot read counts as nothing: this is
/// for telling a person roughly how much will be freed, not for accounting.
pub fn size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    // A link is not followed: it would count what is somewhere else, or loop.
    if !meta.is_dir() {
        return 0;
    }
    fs::read_dir(path)
        .map(|entries| entries.flatten().map(|e| size(&e.path())).sum())
        .unwrap_or(0)
}

/// Where a project's room goes: the folder as a whole, and the folders that grow by themselves.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Usage {
    pub total: u64,
    /// Rust build files, when there are any.
    pub target: Option<u64>,
    /// JavaScript dependencies, when there are any.
    pub node_modules: Option<u64>,
    pub git: Option<u64>,
}

/// Measures a project. It walks every file, which takes a moment on a big `target/`: run it away
/// from the screen.
pub fn usage(root: &Path) -> Usage {
    let measure = |rel: &str| {
        let dir = root.join(rel);
        dir.is_dir().then(|| size(&dir))
    };
    Usage {
        total: size(root),
        // A workspace has one `target/`; separate crates have their own.
        target: [
            measure("target"),
            measure("backend/target"),
            measure("frontend/target"),
        ]
        .into_iter()
        .flatten()
        .reduce(|a, b| a + b),
        node_modules: [measure("node_modules"), measure("frontend/node_modules")]
            .into_iter()
            .flatten()
            .reduce(|a, b| a + b),
        git: measure(".git"),
    }
}

/// `1.2 GB`, `340 MB`, `12 KB`.
pub fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn sizes_are_added_up_under_a_folder_and_read_in_a_human_unit() {
        let root = env::temp_dir().join(format!("stackup-disk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("a/b")).unwrap();
        fs::write(root.join("one"), vec![0u8; 1000]).unwrap();
        fs::write(root.join("a/b/two"), vec![0u8; 2000]).unwrap();
        assert_eq!(size(&root), 3000);
        assert_eq!(size(&root.join("missing")), 0);

        assert_eq!(human(512), "512 B");
        assert_eq!(human(1536), "1.5 KB");
        assert_eq!(human(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(human(3 * 1024 * 1024 * 1024 + 300 * 1024 * 1024), "3.3 GB");
        assert_eq!(human(250 * 1024 * 1024), "250 MB");
    }

    #[test]
    fn a_project_is_measured_as_a_whole_and_by_what_grows() {
        let root = env::temp_dir().join(format!("stackup-usage-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("target/debug")).unwrap();
        fs::create_dir_all(root.join("frontend/node_modules/x")).unwrap();
        fs::write(root.join("target/debug/a"), vec![0u8; 4000]).unwrap();
        fs::write(root.join("frontend/node_modules/x/b"), vec![0u8; 1000]).unwrap();
        fs::write(root.join("src.rs"), vec![0u8; 100]).unwrap();
        let u = usage(&root);
        assert_eq!(u.total, 5100);
        assert_eq!(u.target, Some(4000));
        assert_eq!(u.node_modules, Some(1000));
        assert_eq!(u.git, None);
    }
}
