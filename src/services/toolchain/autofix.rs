//! Reading why something failed, and what would put it right. Only failures that are recognised
//! get a fix, and a fix is a few plain commands that are shown before they run.
use std::path::Path;

use super::machine::Machine;
use crate::models::Step;

/// Something that went wrong, and the steps that fix it. They are followed by running the
/// failed steps again.
pub struct Fix {
    /// One per kind of problem, so the same fix is never offered twice in a row.
    pub id: &'static str,
    pub problem: String,
    pub steps: Vec<Step>,
    /// Set when the problem is a Rust that is too old: the version it needs, so the person can
    /// be asked which Rust to get.
    pub needs_rust: Option<(u32, u32)>,
}

/// `step` run with `toolchain`: a cargo command gets `+toolchain` in front of its arguments.
pub fn with_toolchain(step: &Step, toolchain: &str) -> Step {
    let mut step = step.clone();
    if step.prog == "cargo" && !step.args.first().is_some_and(|a| a.starts_with('+')) {
        step.args.insert(0, format!("+{toolchain}"));
    }
    step
}

/// "it requires rustc 1.99 or newer, while ..." -> "1.99".
fn needed_rust(log: &str) -> Option<String> {
    let after = log.split("requires rustc ").nth(1)?;
    let version = after.split_whitespace().next()?;
    version
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.')
        .then(|| version.to_string())
}

fn rust_too_old(log: &str, cwd: &Path, m: &dyn Machine) -> Option<Fix> {
    let needed = needed_rust(log)?;
    // Without rustup there is nothing here that can update Rust.
    if !m.has("rustup") {
        return None;
    }
    Some(Fix {
        id: "rust-too-old",
        problem: format!("Your Rust is too old: this needs {needed} or newer"),
        steps: vec![Step::new(
            "update Rust",
            cwd,
            "rustup",
            &["update", "stable"],
        )],
        needs_rust: super::machine::rust_version(&needed),
    })
}

fn docker_not_running(log: &str, cwd: &Path, os: &str) -> Option<Fix> {
    let log = log.to_lowercase();
    let down = [
        "cannot connect to the docker daemon",
        "is the docker daemon running",
        "docker daemon is not running",
        "error during connect",
    ]
    .iter()
    .any(|p| log.contains(p));
    // Starting Docker Desktop is something an app can do on a Mac; on Linux it takes sudo.
    if !down || os != "macos" {
        return None;
    }
    let wait = "i=0; while [ $i -lt 90 ]; do docker info >/dev/null 2>&1 && exit 0; i=$((i+1)); sleep 2; done; exit 1";
    Some(Fix {
        id: "docker-not-running",
        problem: "Docker is not running".into(),
        steps: vec![
            Step::new("start Docker Desktop", cwd, "open", &["-a", "Docker"]),
            Step::new("wait for Docker to be ready", cwd, "sh", &["-c", wait]),
        ],
        needs_rust: None,
    })
}

fn env_missing(log: &str, root: &Path) -> Option<Fix> {
    let log = log.to_lowercase();
    let missing = log.contains("env file") && log.contains("not found");
    if !missing || root.join(".env").exists() || !root.join(".env.local").is_file() {
        return None;
    }
    Some(Fix {
        id: "env-missing",
        problem: ".env is missing: compose reads it".into(),
        steps: vec![crate::services::projects::internal::copy(
            "make the local tier the current one",
            root,
            ".env.local",
            ".env",
        )],
        needs_rust: None,
    })
}

/// Postgres 18 stores its data in a versioned folder under `/var/lib/postgresql`, and refuses to
/// start when a volume is mounted at the old `/var/lib/postgresql/data`. Projects made while
/// `postgres:alpine` was 17 have that old mount.
fn postgres_18_volume(log: &str, root: &Path) -> Option<Fix> {
    if !log.contains("in 18+, these Docker images are configured to store database data") {
        return None;
    }
    let compose = std::fs::read_to_string(root.join("docker-compose.yml")).ok()?;
    if !compose.contains(":/var/lib/postgresql/data") {
        return None;
    }
    let fix = "sed 's#:/var/lib/postgresql/data#:/var/lib/postgresql#' docker-compose.yml > docker-compose.yml.tmp && mv docker-compose.yml.tmp docker-compose.yml";
    Some(Fix {
        id: "postgres-18-volume",
        problem: "Postgres 18 keeps its data one folder up: the volume is mounted at the old path"
            .into(),
        steps: vec![Step::new(
            "mount the Postgres volume at /var/lib/postgresql",
            root,
            "sh",
            &["-c", fix],
        )],
        needs_rust: None,
    })
}

/// What to do about `log`, the output of a failed run, if it is a failure we know. `root` is the
/// project's folder, or the current one.
pub fn diagnose(log: &str, root: &Path, os: &str, m: &dyn Machine) -> Option<Fix> {
    rust_too_old(log, root, m)
        .or_else(|| postgres_18_volume(log, root))
        .or_else(|| docker_not_running(log, root, os))
        .or_else(|| env_missing(log, root))
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;
    use crate::services::toolchain::machine::fake;

    const OLD_RUST: &str = "error: cannot install package `topcoat-cli 0.10.0`, it requires rustc 1.99 or newer, while the currently active rustc version is 1.97.1";

    #[test]
    fn an_old_rust_is_updated_through_rustup() {
        let fix = diagnose(
            OLD_RUST,
            Path::new("."),
            "linux",
            &fake(&["cargo", "rustup"]),
        )
        .unwrap();
        assert_eq!(fix.id, "rust-too-old");
        assert!(fix.problem.contains("1.99"), "{}", fix.problem);
        let steps: Vec<String> = fix.steps.iter().map(Step::command_line).collect();
        assert_eq!(steps, ["rustup update stable"]);
    }

    #[test]
    fn without_rustup_an_old_rust_has_no_fix_from_here() {
        assert!(diagnose(OLD_RUST, Path::new("."), "linux", &fake(&["cargo"])).is_none());
    }

    #[test]
    fn docker_is_started_on_a_mac_and_waited_for() {
        let log = "Cannot connect to the Docker daemon at unix:///var/run/docker.sock. Is the docker daemon running?";
        let fix = diagnose(log, Path::new("."), "macos", &fake(&[])).unwrap();
        assert_eq!(fix.id, "docker-not-running");
        let steps: Vec<String> = fix.steps.iter().map(Step::command_line).collect();
        assert_eq!(steps[0], "open -a Docker");
        assert!(
            steps[1].contains("docker info"),
            "it waits until Docker answers"
        );
        // Starting the service on Linux takes sudo, which this does not do.
        assert!(diagnose(log, Path::new("."), "linux", &fake(&[])).is_none());
    }

    #[test]
    fn a_missing_env_is_made_from_the_local_tier_when_there_is_one() {
        let root = env::temp_dir().join(format!("stackup-autofix-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let log =
            "env file /somewhere/.env not found: stat /somewhere/.env: no such file or directory";
        assert!(
            diagnose(log, &root, "linux", &fake(&[])).is_none(),
            "no .env.local to copy"
        );

        fs::write(root.join(".env.local"), "A=1\n").unwrap();
        let fix = diagnose(log, &root, "linux", &fake(&[])).unwrap();
        assert_eq!(
            fix.steps[0].args,
            ["internal", "copy", ".env.local", ".env"]
        );

        fs::write(root.join(".env"), "").unwrap();
        assert!(
            diagnose(log, &root, "linux", &fake(&[])).is_none(),
            ".env is there: it was not that"
        );
    }

    #[test]
    fn an_error_that_is_not_recognised_gets_no_fix() {
        let log = "error: could not compile `foo` due to 3 previous errors";
        assert!(diagnose(log, Path::new("."), "macos", &fake(&["rustup"])).is_none());
    }

    #[test]
    fn the_old_postgres_mount_is_rewritten_in_the_compose_file() {
        let root = env::temp_dir().join(format!("stackup-pg18-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let compose = "services:\n  postgres:\n    image: postgres:alpine\n    volumes: [pgdata:/var/lib/postgresql/data]\nvolumes:\n  pgdata:\n";
        fs::write(root.join("docker-compose.yml"), compose).unwrap();
        let log = "postgres-1  | Error: in 18+, these Docker images are configured to store database data in a\npostgres-1 exited with code 1";

        let fix = diagnose(log, &root, "linux", &fake(&[])).expect("recognised");
        assert_eq!(fix.id, "postgres-18-volume");

        // Run the real fix on the real file, in a folder of its own.
        let step = &fix.steps[0];
        let status = std::process::Command::new(&step.prog)
            .args(&step.args)
            .current_dir(&step.cwd)
            .status()
            .unwrap();
        assert!(status.success());
        let after = fs::read_to_string(root.join("docker-compose.yml")).unwrap();
        assert!(after.contains("[pgdata:/var/lib/postgresql]"), "{after}");
        assert!(!after.contains("/data]"));
        assert!(!root.join("docker-compose.yml.tmp").exists());

        // Fixed, the same log is no longer that problem.
        assert!(diagnose(log, &root, "linux", &fake(&[])).is_none());
    }

    #[test]
    fn a_postgres_failure_for_another_reason_is_not_treated_as_that_one() {
        let log =
            "postgres-1  | FATAL: password authentication failed\npostgres-1 exited with code 1";
        assert!(diagnose(log, Path::new("."), "linux", &fake(&[])).is_none());
    }
}
