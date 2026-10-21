//! Which project a command means: `--path`, `--project NAME`, or the one the shell is inside.
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use super::registry;
use crate::models::Project;

/// The folder holding `docker-compose.yml`: this one, or the nearest one above.
pub fn find_root(from: &Path) -> Option<PathBuf> {
    from.ancestors()
        .find(|d| d.join("docker-compose.yml").is_file())
        .map(Path::to_path_buf)
}

/// A project that was not built with stackup has no entry; it still works, named after its folder.
fn at(registry_dir: &Path, path: &Path) -> Result<Project> {
    let path = crate::clients::os::canonical(path)?;
    let known = registry::load(registry_dir)?;
    if let Some(p) = known.into_iter().find(|p| p.path == path) {
        return Ok(p);
    }
    let name = path
        .file_name()
        .map_or_else(|| "project".into(), |n| n.to_string_lossy().into_owned());
    Ok(Project {
        name,
        path,
        host: None,
    })
}

pub fn resolve(
    registry_dir: &Path,
    project: Option<&str>,
    path: Option<&Path>,
    cwd: &Path,
) -> Result<Project> {
    if let Some(path) = path {
        if !path.join("docker-compose.yml").is_file() {
            bail!("{} has no docker-compose.yml", path.display());
        }
        return at(registry_dir, path);
    }
    if let Some(name) = project {
        let matches: Vec<Project> = registry::load(registry_dir)?
            .into_iter()
            .filter(|p| p.name == name)
            .collect();
        return match matches.as_slice() {
            [one] => Ok(one.clone()),
            [] => bail!("no project named `{name}`: `stackup projects` lists them"),
            many => {
                let paths: Vec<String> =
                    many.iter().map(|p| p.path.display().to_string()).collect();
                bail!(
                    "`{name}` is more than one project: use --path ({})",
                    paths.join(", ")
                )
            }
        };
    }
    match find_root(cwd) {
        Some(root) => at(registry_dir, &root),
        None => bail!("not inside a project: cd into one, or use --project NAME or --path DIR"),
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("stackup-resolve-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn the_nearest_compose_file_above_is_the_project() {
        let root = scratch("up");
        fs::write(root.join("docker-compose.yml"), "").unwrap();
        fs::create_dir_all(root.join("backend/src")).unwrap();
        assert_eq!(find_root(&root.join("backend/src")), Some(root.clone()));
        let reg = scratch("up-reg");
        let p = resolve(&reg, None, None, &root.join("backend")).unwrap();
        assert_eq!(p.path, root);
    }

    #[test]
    fn a_name_finds_the_remembered_project_and_its_host() {
        let root = scratch("named");
        fs::write(root.join("docker-compose.yml"), "").unwrap();
        let reg = scratch("named-reg");
        registry::add(&reg, "shop", &root).unwrap();
        registry::set_host(&reg, &root, "ops@example.com").unwrap();
        let p = resolve(&reg, Some("shop"), None, Path::new("/")).unwrap();
        assert_eq!(p.path, root);
        assert_eq!(p.host.as_deref(), Some("ops@example.com"));
        assert!(resolve(&reg, Some("nope"), None, Path::new("/")).is_err());
    }

    #[test]
    fn two_projects_with_one_name_must_be_told_apart() {
        let (a, b) = (scratch("dup-a"), scratch("dup-b"));
        let reg = scratch("dup-reg");
        registry::add(&reg, "shop", &a).unwrap();
        registry::add(&reg, "shop", &b).unwrap();
        let err = resolve(&reg, Some("shop"), None, Path::new("/")).unwrap_err();
        assert!(err.to_string().contains("--path"), "{err}");
    }
}
