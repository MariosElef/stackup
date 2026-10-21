use std::path::PathBuf;

/// One program to run. An action is a list of these, so the command line and the menu run the
/// same thing.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub label: String,
    pub prog: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
}

impl Step {
    pub fn new(label: &str, cwd: &std::path::Path, prog: &str, args: &[&str]) -> Self {
        Self {
            label: label.to_string(),
            prog: prog.to_string(),
            args: args.iter().map(|a| (*a).to_string()).collect(),
            env: vec![],
            cwd: cwd.to_path_buf(),
        }
    }

    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_string(), value.to_string()));
        self
    }

    /// The program as a person reads it: a path to stackup itself is shown as just its name.
    fn shown_prog(&self) -> String {
        let path = std::path::Path::new(&self.prog);
        if path.is_absolute() {
            path.file_stem()
                .map_or_else(|| self.prog.clone(), |n| n.to_string_lossy().into_owned())
        } else {
            self.prog.clone()
        }
    }

    /// The command as it would be typed: arguments with spaces in single quotes.
    pub fn command_line(&self) -> String {
        let env: String = self.env.iter().map(|(k, v)| format!("{k}={v} ")).collect();
        let args: Vec<String> = self
            .args
            .iter()
            .map(|a| {
                if a.contains(char::is_whitespace) {
                    format!("'{a}'")
                } else {
                    a.clone()
                }
            })
            .collect();
        format!("{env}{} {}", self.shown_prog(), args.join(" "))
            .trim_end()
            .to_string()
    }
}

/// The values an action may need. Anything not given has a default or is asked for.
#[derive(Clone, Debug, Default)]
pub struct Args {
    pub host: Option<String>,
    pub tier: Option<String>,
    pub platform: Option<String>,
    pub dir: Option<String>,
    pub detach: bool,
    pub service: Option<String>,
    /// `private` or `public`, for a new GitHub repository.
    pub visibility: Option<String>,
    /// `github` or `gitlab`: also delete the project's repository there, when removing it.
    pub remove_remote: Option<String>,
    /// A volume: its name, or a `./folder` to mount.
    pub volume: Option<String>,
    /// Where a volume appears inside the container.
    pub mount: Option<String>,
    /// A backup file in the project's `backups/` folder.
    pub backup: Option<String>,
}
