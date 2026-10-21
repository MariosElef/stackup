//! Everything that is different on macOS, Linux and Windows, in one place. The rest of the code
//! asks this file (where is home, how do I open an address, which shell runs a script) and never
//! checks the operating system itself.
use std::{
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};

/// Whether this is Windows.
pub const WINDOWS: bool = cfg!(windows);

/// The person's home folder: `$HOME`, or `%USERPROFILE%` on Windows.
pub fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Where stackup keeps its files: `$STACKUP_HOME`, else `%APPDATA%\stackup` on Windows, else
/// `~/.config/stackup`.
pub fn config_dir() -> PathBuf {
    if let Some(dir) = env::var_os("STACKUP_HOME") {
        return PathBuf::from(dir);
    }
    config_dir_in(
        WINDOWS,
        env::var_os("APPDATA").map(PathBuf::from),
        home_dir(),
    )
}

fn config_dir_in(windows: bool, appdata: Option<PathBuf>, home: Option<PathBuf>) -> PathBuf {
    match (windows, appdata, home) {
        (true, Some(appdata), _) => appdata.join("stackup"),
        (_, _, Some(home)) => home.join(".config").join("stackup"),
        _ => PathBuf::from(".").join(".config").join("stackup"),
    }
}

/// `~/rest` or `~\rest` as a path under home: what a person types for their home folder.
pub fn expand_home(typed: &str) -> PathBuf {
    let rest = typed
        .strip_prefix("~/")
        .or_else(|| typed.strip_prefix("~\\"));
    match (rest, home_dir()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(typed),
    }
}

/// Makes a file readable by its owner only. Windows keeps a file in the user's own profile
/// private already, so there is nothing to do there.
pub fn make_private(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Makes a script runnable. Windows has no such bit: git keeps it in its index instead.
pub fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Random bytes from the operating system, for passwords.
pub fn random_bytes(buf: &mut [u8]) -> Result<()> {
    getrandom::fill(buf)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("could not read random bytes to make passwords with")
}

/// A path as the programs we start expect it: on Windows `canonicalize` gives `\\?\C:\x`, which
/// `cmd` and many tools refuse, so the prefix is dropped.
pub fn clean_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        // `\\?\UNC\server\share` is a network path: it keeps its two leading backslashes.
        if let Some(unc) = rest.strip_prefix(r"UNC\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        return PathBuf::from(rest);
    }
    path
}

/// `fs::canonicalize`, with a path the other programs can use.
pub fn canonical(path: &Path) -> io::Result<PathBuf> {
    fs::canonicalize(path).map(clean_path)
}

/// The file a program name stands for. On Windows `pnpm` is `pnpm.cmd`, and starting a program
/// by its bare name only finds `.exe`, so the name is looked up along `PATH` with each of
/// `PATHEXT`. Elsewhere the name is used as it is.
pub fn program(name: &str) -> OsString {
    if name == "sh" {
        return shell().into_os_string();
    }
    if !WINDOWS {
        return OsString::from(name);
    }
    let path = env::var_os("PATH").unwrap_or_default();
    let exts = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    find_program(name, &path, &exts, &|p| p.is_file())
        .map_or_else(|| OsString::from(name), PathBuf::into_os_string)
}

/// The first `name` + extension found in the `PATH` folders. `exists` is asked, so this can be
/// tested without a Windows machine.
pub fn find_program(
    name: &str,
    path: &std::ffi::OsStr,
    pathext: &str,
    exists: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    // A name that already has a path or an extension is taken as it is.
    if name.contains(['/', '\\']) || Path::new(name).extension().is_some() {
        return None;
    }
    for dir in env::split_paths(path) {
        for ext in pathext.split(';').filter(|e| !e.is_empty()) {
            let candidate = dir.join(format!("{name}{}", ext.to_lowercase()));
            if exists(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// The shell that runs a script. `sh` on a Mac or Linux; on Windows the one that comes with Git
/// for Windows, which stackup needs anyway, next to the `cp`, `grep` and `rm` the scripts use.
pub fn shell() -> PathBuf {
    if !WINDOWS {
        return PathBuf::from("sh");
    }
    Command::new(program_for_git())
        .arg("--exec-path")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|exec| git_sh(Path::new(exec.trim()), &|p| p.is_file()))
        .unwrap_or_else(|| PathBuf::from("sh"))
}

fn program_for_git() -> OsString {
    OsString::from("git")
}

/// Git for Windows keeps `sh.exe` at `<git>\usr\bin` and `<git>\bin`; its exec path is
/// `<git>\mingw64\libexec\git-core`, three folders down.
pub fn git_sh(exec_path: &Path, exists: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    let root = exec_path.ancestors().nth(3)?;
    [root.join("usr").join("bin"), root.join("bin")]
        .into_iter()
        .map(|dir| dir.join("sh.exe"))
        .find(|p| exists(p))
}

/// Whether the shell scripts run in is on this machine. Always, except on a Windows machine
/// without Git for Windows.
pub fn shell_found() -> bool {
    !WINDOWS || shell().is_absolute()
}

/// Where to get the shell on Windows, for the message that says it is missing.
pub const SHELL_HELP: &str = "Git for Windows is needed (it brings the shell stackup's scripts run in): winget install Git.Git, or download it from https://git-scm.com/download/win";

/// A `Command` for a program, found the way this operating system finds it.
pub fn command(name: &str) -> Command {
    Command::new(program(name))
}

/// What opens an address in the browser: the program and the arguments before the address.
pub fn opener() -> (&'static str, &'static [&'static str]) {
    if cfg!(target_os = "macos") {
        ("open", &[])
    } else if WINDOWS {
        // `start` is a cmd built-in and mangles `&` in an address; this is what it calls.
        ("rundll32", &["url.dll,FileProtocolHandler"])
    } else {
        ("xdg-open", &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shell_is_always_found_off_windows_and_the_help_names_the_download() {
        if !WINDOWS {
            assert!(shell_found());
        }
        assert!(SHELL_HELP.contains("winget install Git.Git"));
        assert!(SHELL_HELP.contains("git-scm.com/download/win"));
    }

    #[test]
    fn the_config_folder_is_appdata_on_windows_and_dot_config_elsewhere() {
        let home = Some(PathBuf::from("/h"));
        assert_eq!(
            config_dir_in(true, Some(PathBuf::from("/a")), home.clone()),
            PathBuf::from("/a/stackup")
        );
        assert_eq!(
            config_dir_in(false, Some(PathBuf::from("/a")), home.clone()),
            PathBuf::from("/h/.config/stackup")
        );
        assert_eq!(
            config_dir_in(true, None, home),
            PathBuf::from("/h/.config/stackup"),
            "no APPDATA: home is the next best"
        );
    }

    #[test]
    fn the_windows_verbatim_prefix_is_dropped_from_a_path() {
        assert_eq!(
            clean_path(PathBuf::from(r"\\?\C:\Users\me\shop")),
            PathBuf::from(r"C:\Users\me\shop")
        );
        assert_eq!(
            clean_path(PathBuf::from(r"\\?\UNC\server\share\shop")),
            PathBuf::from(r"\\server\share\shop")
        );
        assert_eq!(
            clean_path(PathBuf::from("/tmp/shop")),
            PathBuf::from("/tmp/shop")
        );
    }

    #[test]
    fn a_program_is_found_with_each_extension_in_turn_along_the_path() {
        let path = env::join_paths(["/nope", "/tools"]).unwrap();
        let has = |p: &Path| p == Path::new("/tools/pnpm.cmd");
        assert_eq!(
            find_program("pnpm", &path, ".COM;.EXE;.BAT;.CMD", &has),
            Some(PathBuf::from("/tools/pnpm.cmd"))
        );
        // Already a path, or already has an extension: left as it is.
        assert_eq!(find_program("C:/x/pnpm", &path, ".CMD", &has), None);
        assert_eq!(find_program("pnpm.cmd", &path, ".CMD", &has), None);
        assert_eq!(find_program("missing", &path, ".CMD", &has), None);
    }

    #[test]
    fn git_for_windows_keeps_its_sh_three_folders_above_its_exec_path() {
        let exec = Path::new("C:/Program Files/Git/mingw64/libexec/git-core");
        let at_usr = |p: &Path| p == Path::new("C:/Program Files/Git/usr/bin/sh.exe");
        assert_eq!(
            git_sh(exec, &at_usr),
            Some(PathBuf::from("C:/Program Files/Git/usr/bin/sh.exe"))
        );
        let at_bin = |p: &Path| p == Path::new("C:/Program Files/Git/bin/sh.exe");
        assert_eq!(
            git_sh(exec, &at_bin),
            Some(PathBuf::from("C:/Program Files/Git/bin/sh.exe"))
        );
        assert_eq!(git_sh(exec, &|_| false), None);
    }

    #[test]
    fn home_can_be_typed_with_a_tilde_and_either_slash() {
        let home = home_dir().unwrap_or_default();
        assert_eq!(expand_home("~/work"), home.join("work"));
        assert_eq!(expand_home("~\\work"), home.join("work"));
        assert_eq!(expand_home("/abs/path"), PathBuf::from("/abs/path"));
    }

    #[test]
    fn random_bytes_are_filled_and_differ() {
        let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
        random_bytes(&mut a).unwrap();
        random_bytes(&mut b).unwrap();
        assert_ne!(a, [0u8; 32]);
        assert_ne!(a, b);
    }

    #[cfg(unix)]
    #[test]
    fn a_private_file_is_readable_by_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let file = env::temp_dir().join(format!("stackup-os-{}", std::process::id()));
        fs::write(&file, "x").unwrap();
        make_private(&file).unwrap();
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
        make_executable(&file).unwrap();
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }

    #[test]
    fn the_browser_opener_is_the_systems_own() {
        let (prog, _) = opener();
        assert!(["open", "xdg-open", "rundll32"].contains(&prog));
    }
}
