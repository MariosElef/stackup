//! Getting this machine ready to push to GitHub: the GitHub CLI signed in, and an SSH key that
//! GitHub knows. Once it is done every program here, in the menu's terminal or any other, uses it
//! the way it would if it had always been there: the key lives in `~/.ssh`, `ssh` is told to use
//! it for github.com, and `gh` keeps its own login.
use std::path::{Path, PathBuf};

use crate::models::Step;

/// What is already in place.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Github {
    /// `gh` is signed in to github.com.
    pub signed_in: bool,
    /// The private key of an SSH key pair that exists here, if any.
    pub key: Option<PathBuf>,
}

impl Github {
    pub fn ready(&self) -> bool {
        self.signed_in && self.key.is_some()
    }
}

/// The key pairs looked for, in the order a person is likely to have used them.
const KEYS: [&str; 3] = ["id_ed25519", "id_ecdsa", "id_rsa"];

/// The key pair in `ssh_dir` to use, when there is one (both halves present).
pub fn find_key(ssh_dir: &Path) -> Option<PathBuf> {
    KEYS.iter().map(|name| ssh_dir.join(name)).find(|private| {
        let mut public = private.clone().into_os_string();
        public.push(".pub");
        private.is_file() && Path::new(&public).is_file()
    })
}

/// Asks this machine: is `gh` signed in, and which key is there.
pub fn read(home: &Path) -> Github {
    let signed_in = crate::clients::os::command("gh")
        .args(["auth", "status", "--hostname", "github.com"])
        .output()
        .is_ok_and(|o| o.status.success());
    Github {
        signed_in,
        key: find_key(&home.join(".ssh")),
    }
}

/// `sh -c SCRIPT sh ARGS...`: the arguments reach the script as `$1`, `$2`..., never as part of
/// its text, so a name with a space or a quote in it cannot change what runs.
fn script(label: &str, dir: &Path, text: &str, args: &[&str]) -> Step {
    let mut all = vec!["-c", text, "sh"];
    all.extend(args);
    Step::new(label, dir, "sh", &all)
}

/// The commands that make `state` ready, each skipped when it is already done. `email` names the
/// key and `host` the machine on GitHub's list of keys.
pub fn setup_steps(state: &Github, home: &Path, email: &str, host: &str) -> Vec<Step> {
    let ssh = home.join(".ssh");
    let ssh_text = ssh.to_string_lossy().into_owned();
    let mut steps = vec![];

    let key = match &state.key {
        Some(key) => key.clone(),
        None => {
            let key = ssh.join("id_ed25519");
            steps.push(script(
                "create the ~/.ssh folder",
                home,
                "mkdir -p \"$1\" && chmod 700 \"$1\"",
                &[&ssh_text],
            ));
            // No passphrase: a push never stops to ask. It can be added later with ssh-keygen -p.
            steps.push(Step::new(
                "create an SSH key",
                home,
                "ssh-keygen",
                &[
                    "-t",
                    "ed25519",
                    "-N",
                    "",
                    "-C",
                    email,
                    "-f",
                    &key.to_string_lossy(),
                ],
            ));
            key
        }
    };
    let key_text = key.to_string_lossy().into_owned();
    let public = format!("{key_text}.pub");

    if state.signed_in {
        // A login made without the scope to add keys cannot add one.
        steps.push(Step::new(
            "let gh add SSH keys",
            home,
            "gh",
            &[
                "auth",
                "refresh",
                "--hostname",
                "github.com",
                "--scopes",
                "admin:public_key",
            ],
        ));
    } else {
        steps.push(Step::new(
            "sign in to GitHub",
            home,
            "gh",
            &[
                "auth",
                "login",
                "--hostname",
                "github.com",
                "--git-protocol",
                "ssh",
                "--web",
                "--scopes",
                "admin:public_key",
                "--skip-ssh-key",
            ],
        ));
    }

    let title = format!("stackup on {host}");
    steps.push(script(
        "give GitHub the key",
        home,
        // A key GitHub already has is not a failure.
        "out=$(gh ssh-key add \"$1\" --title \"$2\" 2>&1); code=$?; echo \"$out\"; \
         case \"$out\" in *\"already in use\"*) exit 0;; esac; exit $code",
        &[&public, &title],
    ));

    let config = ssh.join("config").to_string_lossy().into_owned();
    steps.push(script(
        "tell ssh to use it for github.com",
        home,
        // Added once, and only when there is no github.com entry of the person's own.
        "grep -qs '^Host github.com' \"$1\" || { printf '\\nHost github.com\\n  AddKeysToAgent yes\\n  IgnoreUnknown UseKeychain\\n  UseKeychain yes\\n  IdentityFile %s\\n' \"$2\" >> \"$1\"; chmod 600 \"$1\"; }",
        &[&config, &key_text],
    ));

    steps.push(script(
        "check that GitHub lets you in",
        home,
        // `ssh -T` always exits 1; what counts is what GitHub says.
        "ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new -T git@github.com 2>&1 | tee /dev/stderr | grep -q 'successfully authenticated'",
        &[],
    ));
    steps
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    fn home(test: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("stackup-gh-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".ssh")).unwrap();
        dir
    }

    fn labels(steps: &[Step]) -> Vec<&str> {
        steps.iter().map(|s| s.label.as_str()).collect()
    }

    #[test]
    fn ready_needs_both_the_login_and_a_key() {
        let key = Some(PathBuf::from("/h/.ssh/id_ed25519"));
        assert!(
            Github {
                signed_in: true,
                key: key.clone()
            }
            .ready()
        );
        assert!(
            !Github {
                signed_in: true,
                key: None
            }
            .ready()
        );
        assert!(
            !Github {
                signed_in: false,
                key
            }
            .ready()
        );
    }

    #[test]
    fn a_key_counts_only_with_both_halves_and_ed25519_comes_first() {
        let h = home("find");
        let ssh = h.join(".ssh");
        assert_eq!(find_key(&ssh), None);
        fs::write(ssh.join("id_rsa"), "k").unwrap();
        assert_eq!(find_key(&ssh), None, "a private key alone is not a pair");
        fs::write(ssh.join("id_rsa.pub"), "p").unwrap();
        assert_eq!(find_key(&ssh), Some(ssh.join("id_rsa")));
        fs::write(ssh.join("id_ed25519"), "k").unwrap();
        fs::write(ssh.join("id_ed25519.pub"), "p").unwrap();
        assert_eq!(find_key(&ssh), Some(ssh.join("id_ed25519")));
    }

    #[test]
    fn a_fresh_machine_gets_a_key_a_login_the_key_uploaded_and_ssh_told_about_it() {
        let h = home("fresh");
        let steps = setup_steps(&Github::default(), &h, "me@example.com", "my-mac");
        assert_eq!(
            labels(&steps),
            [
                "create the ~/.ssh folder",
                "create an SSH key",
                "sign in to GitHub",
                "give GitHub the key",
                "tell ssh to use it for github.com",
                "check that GitHub lets you in",
            ]
        );
        let keygen = steps[1].command_line();
        assert!(keygen.starts_with("ssh-keygen -t ed25519 -N"), "{keygen}");
        assert!(keygen.contains("me@example.com") && keygen.contains("id_ed25519"));
        let login = steps[2].command_line();
        assert!(
            login.contains("--git-protocol ssh") && login.contains("--web"),
            "{login}"
        );
        assert!(login.contains("admin:public_key"), "it may add the key");
        assert!(steps[3].command_line().contains("stackup on my-mac"));
    }

    #[test]
    fn what_is_already_there_is_not_done_again() {
        let h = home("partial");
        let key = h.join(".ssh/id_ed25519");
        // A key but no login: only the login and what follows.
        let steps = setup_steps(
            &Github {
                signed_in: false,
                key: Some(key.clone()),
            },
            &h,
            "me@example.com",
            "m",
        );
        assert_eq!(labels(&steps)[0], "sign in to GitHub");
        assert!(!labels(&steps).contains(&"create an SSH key"));
        // A login but no key: a key, then the scope to add it, not another login.
        let steps = setup_steps(
            &Github {
                signed_in: true,
                key: None,
            },
            &h,
            "me@example.com",
            "m",
        );
        assert!(labels(&steps).contains(&"create an SSH key"));
        assert!(labels(&steps).contains(&"let gh add SSH keys"));
        assert!(!labels(&steps).contains(&"sign in to GitHub"));
    }

    #[test]
    fn values_reach_the_scripts_as_arguments_never_as_their_text() {
        let h = home("quote");
        let steps = setup_steps(&Github::default(), &h, "a b; rm -rf ~", "host'name");
        let upload = &steps[3];
        assert_eq!(upload.prog, "sh");
        assert!(
            !upload.args[1].contains("host'name"),
            "the script text is fixed"
        );
        assert!(upload.args.iter().any(|a| a.contains("host'name")));
        // The key's comment is one argument to ssh-keygen, not split on spaces.
        assert!(steps[1].args.contains(&"a b; rm -rf ~".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn the_ssh_config_entry_is_added_once_and_leaves_a_hand_written_one_alone() {
        let h = home("config");
        let steps = setup_steps(
            &Github {
                signed_in: true,
                key: Some(h.join(".ssh/id_ed25519")),
            },
            &h,
            "e",
            "m",
        );
        let entry = steps
            .iter()
            .find(|s| s.label == "tell ssh to use it for github.com")
            .unwrap();
        let config = h.join(".ssh/config");
        let run = || {
            let ok = std::process::Command::new(&entry.prog)
                .args(&entry.args)
                .status()
                .unwrap()
                .success();
            assert!(ok);
        };
        run();
        run();
        let text = fs::read_to_string(&config).unwrap();
        assert_eq!(text.matches("Host github.com").count(), 1, "{text}");
        assert!(text.contains("IdentityFile") && text.contains("id_ed25519"));

        // A github.com entry of the person's own is never added to.
        fs::write(&config, "Host github.com\n  User git\n").unwrap();
        run();
        assert_eq!(
            fs::read_to_string(&config).unwrap(),
            "Host github.com\n  User git\n"
        );
    }
}
