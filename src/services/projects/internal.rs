//! Steps that run stackup itself, for the small jobs (copy a file, delete a folder) that a shell
//! does differently on each system: `cp` and `rm` do not exist on Windows. The same step then
//! reads the same everywhere, and stackup does the work the same way.
use std::{env, path::Path};

use crate::models::Step;

/// This program, to be run as a step: by its full path, as a Mac or Windows menu was not
/// started from a shell that knows where it is.
fn exe() -> String {
    // `STACKUP_BIN` names another build to run: the tests, whose own program is a test harness.
    if let Some(path) = env::var_os("STACKUP_BIN") {
        return path.to_string_lossy().into_owned();
    }
    env::current_exe().map_or_else(|_| "stackup".into(), |p| p.to_string_lossy().into_owned())
}

/// Copy `from` over `to`.
pub fn copy(label: &str, cwd: &Path, from: &str, to: &str) -> Step {
    Step::new(label, cwd, &exe(), &["internal", "copy", from, to])
}

/// Delete `path` and everything in it.
pub fn remove(label: &str, cwd: &Path, path: &Path) -> Step {
    Step::new(
        label,
        cwd,
        &exe(),
        &["internal", "remove", &path.to_string_lossy()],
    )
}

/// Print the volumes of the project in `root` as a table.
pub fn volumes(label: &str, root: &Path) -> Step {
    Step::new(
        label,
        root,
        &exe(),
        &["internal", "volumes", &root.to_string_lossy()],
    )
}

/// Print which parts of the project in `root` start, and where they answer: `which` is `backend`,
/// `frontend` or `both`.
pub fn banner(root: &Path, which: &str) -> Step {
    Step::new(
        "where it runs",
        root,
        &exe(),
        &["internal", "banner", &root.to_string_lossy(), which],
    )
}

/// Give the project in `root` the HTTPS proxy for development, when it was made without it.
pub fn https_dev(root: &Path) -> Step {
    Step::new(
        "set up HTTPS for development",
        root,
        &exe(),
        &["internal", "https-dev", &root.to_string_lossy()],
    )
}

/// Make a folder, and the ones above it.
pub fn make_dir(label: &str, cwd: &Path, path: &Path) -> Step {
    Step::new(
        label,
        cwd,
        &exe(),
        &["internal", "make-dir", &path.to_string_lossy()],
    )
}

/// Mount `source` (a volume name, or a `./folder`) at `target` in `service` of the compose file
/// in `cwd`.
pub fn add_volume(label: &str, cwd: &Path, service: &str, source: &str, target: &str) -> Step {
    Step::new(
        label,
        cwd,
        &exe(),
        &["internal", "add-volume", service, source, target],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_and_a_remove_are_stackup_run_with_the_internal_command() {
        let copy = copy("c", Path::new("."), ".env.local", ".env");
        assert_eq!(copy.args, ["internal", "copy", ".env.local", ".env"]);
        let remove = remove("r", Path::new("."), Path::new("/x/shop"));
        assert_eq!(remove.args, ["internal", "remove", "/x/shop"]);
        assert!(!copy.prog.is_empty());
    }
}
