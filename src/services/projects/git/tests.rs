use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::*;
use crate::models::Step;

/// A real git repository in a folder of its own.
fn repo(test: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("stackup-git-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    dir
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn status_is_read_for_the_branch_how_far_it_is_ahead_and_what_changed() {
    let text = "# branch.oid abc\n# branch.head feature/x\n# branch.upstream origin/feature/x\n# branch.ab +2 -1\n1 .M N... 100644 100644 100644 a b src/main.rs\n? notes.txt\n";
    assert_eq!(parse_status(text), ("feature/x".to_string(), 2, 1, 2));
    assert_eq!(
        parse_status("# branch.head main\n"),
        ("main".to_string(), 0, 0, 0),
        "no upstream, nothing changed"
    );
}

#[test]
fn a_real_repository_is_read_before_and_after_a_commit() {
    let dir = repo("read");
    assert_eq!(read(&dir.join("missing")), None, "not a repository");
    let info = read(&dir).unwrap();
    assert_eq!(
        (
            info.branch.as_str(),
            info.dirty,
            info.commits,
            info.remote.clone()
        ),
        ("main", 0, false, None)
    );

    fs::write(dir.join("a.txt"), "one").unwrap();
    assert_eq!(read(&dir).unwrap().dirty, 1, "an untracked file counts");
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "first"]);
    let info = read(&dir).unwrap();
    assert!(info.commits && info.dirty == 0);

    git(
        &dir,
        &["remote", "add", "origin", "git@github.com:me/app.git"],
    );
    assert_eq!(
        read(&dir).unwrap().remote.as_deref(),
        Some("git@github.com:me/app.git")
    );
}

#[test]
fn a_github_address_gives_its_owner_and_name() {
    assert_eq!(
        github_slug("git@github.com:me/app.git").as_deref(),
        Some("me/app")
    );
    assert_eq!(
        github_slug("https://github.com/me/app.git").as_deref(),
        Some("me/app")
    );
    assert_eq!(
        github_slug("https://github.com/me/app").as_deref(),
        Some("me/app")
    );
    assert_eq!(
        github_slug("git@gitlab.com:me/app.git"),
        None,
        "only GitHub"
    );
    assert_eq!(github_slug("https://github.com/me"), None);
}

#[test]
fn creating_a_repo_commits_first_when_there_is_nothing_and_adds_the_remote_when_there_is_none() {
    let root = Path::new("/p");
    let lines = |v: Vec<Step>| v.iter().map(Step::command_line).collect::<Vec<_>>();
    assert_eq!(
        lines(repo_steps(root, "me/app", "private", false, false)),
        [
            "git add -A",
            "git commit -q -m 'Initial commit'",
            "gh repo create me/app --private --source=. --remote=origin --push"
        ]
    );
    assert_eq!(
        lines(repo_steps(root, "me/app", "public", true, true)),
        ["gh repo create me/app --public", "git push -u origin HEAD"],
        "the remote is already set: the repository is made, then pushed to"
    );
}

#[test]
fn a_remote_says_which_service_and_which_repository() {
    assert_eq!(
        hosted("git@github.com:me/app.git"),
        Some((Host::Github, "me/app".into()))
    );
    assert_eq!(
        hosted("https://gitlab.com/group/sub/app"),
        Some((Host::Gitlab, "group/sub/app".into()))
    );
    assert_eq!(
        hosted("git@gitlab.com:me/app.git"),
        Some((Host::Gitlab, "me/app".into()))
    );
    assert_eq!(
        hosted("git@example.com:me/app.git"),
        None,
        "only the two services"
    );
    assert_eq!(hosted("https://github.com/just-one"), None);
}

#[test]
fn deleting_a_repository_asks_github_for_permission_first_and_names_the_repository() {
    let steps = delete_repo_steps(Path::new("."), Host::Github, "me/app");
    let lines: Vec<String> = steps.iter().map(Step::command_line).collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("--scopes delete_repo"), "{}", lines[0]);
    assert_eq!(lines[1], "gh repo delete me/app --yes");
    let steps = delete_repo_steps(Path::new("."), Host::Gitlab, "g/app");
    assert_eq!(steps[0].command_line(), "glab repo delete g/app --yes");
}
