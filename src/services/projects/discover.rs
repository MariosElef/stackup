//! Finding projects that already exist: a folder with a `docker-compose.yml`, or a folder that
//! holds several of them.
use std::{
    fs,
    path::{Path, PathBuf},
};

/// How many folders down to look, below the one given.
const DEPTH: usize = 2;
/// Enough for a work folder; stops a scan of something enormous.
const LIMIT: usize = 200;
/// Folders that never hold a project of their own.
const SKIP: [&str; 5] = ["node_modules", "target", "vendor", "dist", "build"];

fn is_project(dir: &Path) -> bool {
    dir.join("docker-compose.yml").is_file()
}

fn visit(dir: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    if found.len() >= LIMIT {
        return;
    }
    if is_project(dir) {
        // A project's own folders are not more projects.
        found.push(dir.to_path_buf());
        return;
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            !name.starts_with('.') && !SKIP.contains(&name.as_ref())
        })
        .map(|e| e.path())
        .collect();
    children.sort();
    for child in children {
        visit(&child, depth - 1, found);
    }
}

/// The projects at `root`: itself if it is one, else those a couple of folders below it.
pub fn find(root: &Path) -> Vec<PathBuf> {
    let mut found = vec![];
    visit(root, DEPTH, &mut found);
    found
}

/// The name a project is listed under: its folder's.
pub fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || "project".to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    /// A folder tree for one test: each path gets a docker-compose.yml, or is a plain folder
    /// when it ends in `/`.
    fn tree(test: &str, paths: &[&str]) -> PathBuf {
        let root = env::temp_dir().join(format!("stackup-discover-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for p in paths {
            if let Some(dir) = p.strip_suffix('/') {
                fs::create_dir_all(root.join(dir)).unwrap();
            } else {
                fs::create_dir_all(root.join(p)).unwrap();
                fs::write(root.join(p).join("docker-compose.yml"), "").unwrap();
            }
        }
        root.canonicalize().unwrap()
    }

    fn names(found: &[PathBuf]) -> Vec<String> {
        found.iter().map(|p| name_of(p)).collect()
    }

    #[test]
    fn a_project_folder_is_itself_the_one_found() {
        let root = tree("one", &["shop"]);
        let found = find(&root.join("shop"));
        assert_eq!(found, [root.join("shop")]);
    }

    #[test]
    fn a_folder_of_projects_yields_each_of_them() {
        let root = tree("many", &["shop", "blog", "notes/"]);
        assert_eq!(names(&find(&root)), ["blog", "shop"]);
    }

    #[test]
    fn projects_a_level_further_down_are_found_but_not_deeper() {
        let root = tree(
            "deep",
            &["clients/acme", "clients/globex", "a/b/c/too-deep"],
        );
        assert_eq!(names(&find(&root)), ["acme", "globex"]);
    }

    #[test]
    fn hidden_and_dependency_folders_are_not_searched() {
        let root = tree(
            "skip",
            &["app", "node_modules/pkg", ".hidden/secret", "target/x"],
        );
        assert_eq!(names(&find(&root)), ["app"]);
    }

    #[test]
    fn a_project_does_not_yield_the_projects_inside_it() {
        let root = tree("nested", &["mono", "mono/frontend"]);
        assert_eq!(names(&find(&root)), ["mono"]);
    }

    #[test]
    fn nothing_found_is_an_empty_list() {
        let root = tree("none", &["plain/"]);
        assert!(find(&root).is_empty());
        assert!(find(&root.join("missing")).is_empty());
    }
}
