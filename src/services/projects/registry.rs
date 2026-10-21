//! The projects built with stackup, remembered in `projects.json` so the menu can list them.
use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde_json::{Value, json};

use super::discover;
use crate::models::Project;

/// Where stackup keeps its files: `$STACKUP_HOME`, else the system's own place for them.
pub fn home() -> PathBuf {
    crate::clients::os::config_dir()
}

fn file(dir: &Path) -> PathBuf {
    dir.join("projects.json")
}

pub fn load(dir: &Path) -> Result<Vec<Project>> {
    let text = match fs::read_to_string(file(dir)) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e).context("could not read the list of projects"),
    };
    let json: Value = serde_json::from_str(&text).context("projects.json is not valid JSON")?;
    Ok(json["projects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            Some(Project {
                name: p["name"].as_str()?.to_string(),
                path: PathBuf::from(p["path"].as_str()?),
                host: p["host"].as_str().map(str::to_string),
            })
        })
        .collect())
}

pub fn save(dir: &Path, projects: &[Project]) -> Result<()> {
    fs::create_dir_all(dir)?;
    let list: Vec<Value> = projects
        .iter()
        .map(|p| json!({ "name": p.name, "path": p.path, "host": p.host }))
        .collect();
    let text = serde_json::to_string_pretty(&json!({ "projects": list }))? + "\n";
    // Written beside the file and renamed, so a crash never leaves half a list.
    let tmp = dir.join("projects.json.tmp");
    fs::write(&tmp, text)?;
    fs::rename(tmp, file(dir))?;
    Ok(())
}

/// Remembers a project. One folder is one project: adding it again keeps its host.
pub fn add(dir: &Path, name: &str, path: &Path) -> Result<()> {
    let mut all = load(dir)?;
    let host = all
        .iter()
        .find(|p| p.path == path)
        .and_then(|p| p.host.clone());
    all.retain(|p| p.path != path);
    all.push(Project {
        name: name.to_string(),
        path: path.to_path_buf(),
        host,
    });
    save(dir, &all)
}

/// What adding the projects at or just below a folder did.
#[derive(Debug, Default, PartialEq)]
pub struct Added {
    /// Newly listed.
    pub added: Vec<String>,
    /// Already on the list: left exactly as they were, name and host included.
    pub known: Vec<String>,
}

/// Adds every project at or just below `from` that is not listed yet.
pub fn add_found(dir: &Path, from: &Path) -> Result<Added> {
    let listed = load(dir)?;
    let mut result = Added::default();
    // Folders are compared by their real path, so a link to a project is the project.
    let from = crate::clients::os::canonical(from).unwrap_or_else(|_| from.to_path_buf());
    for path in discover::find(&from) {
        match listed.iter().find(|p| p.path == path) {
            Some(existing) => result.known.push(existing.name.clone()),
            None => {
                let name = discover::name_of(&path);
                add(dir, &name, &path)?;
                result.added.push(name);
            }
        }
    }
    Ok(result)
}

/// Stops listing a project. Its files are left alone.
pub fn forget(dir: &Path, path: &Path) -> Result<()> {
    let mut all = load(dir)?;
    all.retain(|p| p.path != path);
    save(dir, &all)
}

pub fn set_host(dir: &Path, path: &Path, host: &str) -> Result<()> {
    let mut all = load(dir)?;
    if let Some(p) = all.iter_mut().find(|p| p.path == path) {
        p.host = Some(host.to_string());
        save(dir, &all)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("stackup-reg-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn nothing_remembered_is_an_empty_list() {
        assert_eq!(load(&scratch("empty")).unwrap(), vec![]);
    }

    #[test]
    fn projects_are_remembered_by_folder_and_keep_their_host() {
        let dir = scratch("add");
        add(&dir, "shop", Path::new("/work/shop")).unwrap();
        add(&dir, "blog", Path::new("/work/blog")).unwrap();
        set_host(&dir, Path::new("/work/shop"), "ops@example.com").unwrap();
        // Adding the same folder again must not lose where it deploys.
        add(&dir, "shop", Path::new("/work/shop")).unwrap();
        let all = load(&dir).unwrap();
        assert_eq!(all.len(), 2);
        let shop = all.iter().find(|p| p.name == "shop").unwrap();
        assert_eq!(shop.host.as_deref(), Some("ops@example.com"));
    }

    #[test]
    fn forgetting_removes_only_the_list_entry() {
        let dir = scratch("forget");
        add(&dir, "shop", Path::new("/work/shop")).unwrap();
        add(&dir, "blog", Path::new("/work/blog")).unwrap();
        forget(&dir, Path::new("/work/shop")).unwrap();
        let names: Vec<String> = load(&dir).unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["blog"]);
    }

    #[test]
    fn adding_a_folder_lists_new_projects_and_leaves_known_ones_as_they_were() {
        let root = scratch("addfound");
        let (shop, blog) = (root.join("work/Shop"), root.join("work/blog"));
        for p in [&shop, &blog] {
            fs::create_dir_all(p).unwrap();
            fs::write(p.join("docker-compose.yml"), "").unwrap();
        }
        let home = root.join("home");
        // Listed earlier under another name, with a host.
        add(&home, "shop", &shop.canonicalize().unwrap()).unwrap();
        set_host(&home, &shop.canonicalize().unwrap(), "ops@example.com").unwrap();

        let result = add_found(&home, &root.join("work")).unwrap();
        assert_eq!(result.added, ["blog"]);
        assert_eq!(
            result.known,
            ["shop"],
            "reported under the name it is listed by"
        );
        let all = load(&home).unwrap();
        let shop = all.iter().find(|p| p.path.ends_with("Shop")).unwrap();
        assert_eq!(
            (shop.name.as_str(), shop.host.as_deref()),
            ("shop", Some("ops@example.com"))
        );
        assert!(
            add_found(&home, &root.join("empty"))
                .unwrap()
                .added
                .is_empty()
        );
    }
}
