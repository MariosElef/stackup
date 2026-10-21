//! A project's environments: `.env.<tier>` files, with `.env` a copy of the active one.
use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use super::plan::valid_tier;
use crate::clients::os;

/// The tiers a project has, `local`, `dev` and `prod` first, then the rest by name.
pub fn tiers(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return vec![];
    };
    let mut found: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_prefix(".env.").map(str::to_string)
        })
        // The template is not a tier, and neither are leftovers like `.env.local.bak`.
        .filter(|t| t != "example" && valid_tier(t))
        .collect();
    let rank = |t: &str| match t {
        "local" => 0,
        "dev" => 1,
        "prod" => 2,
        _ => 3,
    };
    found.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.cmp(b)));
    found
}

/// How many variables `.env.<tier>` sets: lines that are `KEY=value`, not comments or blanks.
pub fn variable_count(root: &Path, tier: &str) -> usize {
    fs::read_to_string(root.join(format!(".env.{tier}"))).map_or(0, |text| {
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#') && l.contains('='))
            .count()
    })
}

/// The tier `.env` is a copy of: the one whose file has exactly its contents. `custom` when it
/// was edited by hand, `None` when there is no `.env`.
pub fn current(root: &Path) -> Option<String> {
    let active = fs::read(root.join(".env")).ok()?;
    // A project that has only a `.env` has one environment, and it is the local one.
    if tiers(root).is_empty() {
        return Some("local".to_string());
    }
    let same = tiers(root)
        .into_iter()
        .find(|t| fs::read(root.join(format!(".env.{t}"))).is_ok_and(|c| c == active));
    Some(same.unwrap_or_else(|| "custom".to_string()))
}

/// `KEY=value` lines of `.env`, for a program that runs on this machine instead of in Docker.
/// Comments and blank lines are skipped, and one pair of quotes around a value is dropped.
pub fn dotenv(root: &Path) -> Vec<(String, String)> {
    let Ok(text) = fs::read_to_string(root.join(".env")) else {
        return vec![];
    };
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (key, value) = l.split_once('=')?;
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);
            Some((key.trim().to_string(), value.to_string()))
        })
        .collect()
}

/// What making a tier did, for the person to read.
#[derive(Debug, PartialEq)]
pub struct Created {
    pub path: PathBuf,
    /// Variables that were given a new random value.
    pub generated: Vec<String>,
    /// Variables left as the template had them that probably need the person's attention: an
    /// address that still says localhost, or a value that is empty.
    pub review: Vec<String>,
}

/// Whether a variable holds a secret, going by its name.
fn is_secret(key: &str) -> bool {
    ["PASSWORD", "SECRET", "TOKEN"]
        .iter()
        .any(|w| key.to_uppercase().contains(w))
}

/// `length` characters, letters and digits, from the system's source of randomness.
fn random_secret(length: usize) -> Result<String> {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut out = String::with_capacity(length);
    let mut chunk = [0u8; 64];
    while out.len() < length {
        os::random_bytes(&mut chunk)?;
        for &b in &chunk {
            // 62 does not divide 256: bytes past the last whole multiple are redrawn, so no
            // letter is more likely than another.
            if b < 248 && out.len() < length {
                out.push(ALPHABET[usize::from(b) % ALPHABET.len()] as char);
            }
        }
    }
    Ok(out)
}

/// Saves `.env` as `.env.local`, when `.env` is all there is: it becomes the local environment,
/// before another is made beside it. Returns whether it did.
fn save_env_as_local(root: &Path) -> Result<bool> {
    let local = root.join(".env.local");
    if local.exists() || !tiers(root).is_empty() || !root.join(".env").is_file() {
        return Ok(false);
    }
    fs::copy(root.join(".env"), &local)?;
    os::make_private(&local)?;
    Ok(true)
}

/// Makes `.env.<tier>` from `.env.example`: every secret gets a new random value (and the copy of
/// a password inside a connection URL follows it), the rest is left as the template has it. The
/// file is readable only by its owner. Never overwrites a tier that exists.
pub fn create(root: &Path, tier: &str) -> Result<Created> {
    if !valid_tier(tier) || tier == "example" {
        bail!("`{tier}` is not a tier name: use letters, digits and dashes, like staging or prod");
    }
    let path = root.join(format!(".env.{tier}"));
    if path.exists() {
        bail!(".env.{tier} already exists: it is not overwritten");
    }
    // The first extra environment: what `.env` holds is the local one, and it is kept as such.
    let kept = save_env_as_local(root)?;
    if tier == "local" && kept {
        return Ok(Created {
            path,
            generated: vec![],
            review: vec![],
        });
    }
    let template = fs::read_to_string(root.join(".env.example"))
        .context("there is no .env.example to make the tier from")?;

    let mut generated = vec![];
    let mut review = vec![];
    let mut passwords: Vec<(String, String)> = vec![];
    let mut body = vec![format!(
        "# {tier} tier (gitignored). `stackup env {tier}` copies this file to .env."
    )];
    for line in template.lines() {
        let trimmed = line.trim();
        // The template's own header is replaced by this tier's.
        if trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            body.push(line.to_string());
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        if is_secret(key) && !value.is_empty() {
            let secret = random_secret(32)?;
            passwords.push((value.to_string(), secret.clone()));
            generated.push(key.to_string());
            body.push(format!("{key}={secret}"));
        } else {
            if value.is_empty() || value.contains("localhost") {
                review.push(key.to_string());
            }
            body.push(format!("{key}={value}"));
        }
    }
    // A password also sits inside connection URLs: the same new value goes there.
    let mut text = body.join("\n") + "\n";
    for (old, new) in &passwords {
        text = text.replace(&format!(":{old}@"), &format!(":{new}@"));
    }
    fs::write(&path, text)?;
    os::make_private(&path)?;
    Ok(Created {
        path,
        generated,
        review,
    })
}

#[cfg(test)]
mod tests {
    use std::env;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn project(test: &str, files: &[(&str, &str)]) -> PathBuf {
        let root = env::temp_dir().join(format!("stackup-envs-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for (name, body) in files {
            fs::write(root.join(name), body).unwrap();
        }
        root
    }

    const EXAMPLE: &str = "# Template. Copy to .env.<tier>.\n# More template text.\nPOSTGRES_USER=app\nPOSTGRES_PASSWORD=change-me\nPOSTGRES_DB=app\nDATABASE_URL=postgres://app:change-me@localhost:5432/app\nKEYCLOAK_ADMIN_PASSWORD=admin\nAPI_URL=http://localhost:8000\nMAILTRAP_USER=\nBIND_ADDR=0.0.0.0:8000\n";

    #[test]
    fn the_tiers_are_the_env_files_without_the_template_and_in_a_sensible_order() {
        let root = project(
            "tiers",
            &[
                (".env.prod", ""),
                (".env.example", ""),
                (".env.staging", ""),
                (".env.local", ""),
                (".env.dev", ""),
                (".env", ""),
                (".env.local.bak", ""),
                ("README.md", ""),
            ],
        );
        assert_eq!(tiers(&root), ["local", "dev", "prod", "staging"]);
        assert!(tiers(&root.join("missing")).is_empty());
    }

    #[test]
    fn the_current_tier_is_the_one_env_is_a_copy_of() {
        let root = project(
            "current",
            &[
                (".env.local", "A=1\n"),
                (".env.prod", "A=2\n"),
                (".env", "A=2\n"),
            ],
        );
        assert_eq!(current(&root).as_deref(), Some("prod"));
        fs::write(root.join(".env"), "A=changed\n").unwrap();
        assert_eq!(current(&root).as_deref(), Some("custom"), "edited by hand");
        fs::remove_file(root.join(".env")).unwrap();
        assert_eq!(current(&root), None);
    }

    #[test]
    fn dotenv_reads_pairs_and_skips_comments_and_quotes() {
        let root = project(
            "dotenv",
            &[(
                ".env",
                "# a comment\n\nA=1\nB = two words \nC=\"quoted\"\nD='single'\nnot a pair\n",
            )],
        );
        assert_eq!(
            dotenv(&root),
            [
                ("A", "1"),
                ("B", "two words"),
                ("C", "quoted"),
                ("D", "single")
            ]
            .map(|(k, v)| (k.to_string(), v.to_string()))
        );
        assert!(dotenv(&root.join("none")).is_empty());
    }

    #[test]
    fn a_new_tier_gets_fresh_secrets_and_keeps_the_rest_of_the_template() {
        let root = project("create", &[(".env.example", EXAMPLE)]);
        let made = create(&root, "prod").unwrap();
        let text = fs::read_to_string(&made.path).unwrap();

        assert!(
            text.starts_with("# prod tier (gitignored). `stackup env prod`"),
            "{text}"
        );
        assert!(
            !text.contains("Template"),
            "the template's header is replaced"
        );
        assert!(text.contains("POSTGRES_USER=app") && text.contains("BIND_ADDR=0.0.0.0:8000"));
        assert_eq!(
            made.generated,
            ["POSTGRES_PASSWORD", "KEYCLOAK_ADMIN_PASSWORD"]
        );
        assert!(
            !text.contains("change-me") && !text.contains("PASSWORD=admin"),
            "{text}"
        );

        // The password inside the connection URL is the same new one.
        let line = |key: &str| {
            text.lines()
                .find(|l| l.starts_with(key))
                .unwrap()
                .to_string()
        };
        let password = line("POSTGRES_PASSWORD=").replace("POSTGRES_PASSWORD=", "");
        assert_eq!(password.len(), 32);
        assert!(password.chars().all(|c| c.is_ascii_alphanumeric()));
        assert!(
            line("DATABASE_URL=").contains(&format!(":{password}@")),
            "{text}"
        );
        assert_ne!(
            password,
            line("KEYCLOAK_ADMIN_PASSWORD=").replace("KEYCLOAK_ADMIN_PASSWORD=", "")
        );

        // What still says localhost, or is empty, is for the person to look at.
        assert_eq!(made.review, ["DATABASE_URL", "API_URL", "MAILTRAP_USER"]);
    }

    #[test]
    fn two_tiers_never_share_a_secret() {
        let root = project("unique", &[(".env.example", EXAMPLE)]);
        let a = fs::read_to_string(create(&root, "dev").unwrap().path).unwrap();
        let b = fs::read_to_string(create(&root, "prod").unwrap().path).unwrap();
        let pw = |t: &str| {
            t.lines()
                .find(|l| l.starts_with("POSTGRES_PASSWORD="))
                .unwrap()
                .to_string()
        };
        assert_ne!(pw(&a), pw(&b));
    }

    #[test]
    fn the_file_is_private_and_an_existing_tier_is_never_overwritten() {
        let root = project(
            "safe",
            &[(".env.example", EXAMPLE), (".env.dev", "mine=1\n")],
        );
        let made = create(&root, "prod").unwrap();
        assert!(made.path.is_file());
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&made.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let err = create(&root, "dev").unwrap_err();
        assert!(err.to_string().contains("already exists"), "{err}");
        assert_eq!(
            fs::read_to_string(root.join(".env.dev")).unwrap(),
            "mine=1\n",
            "untouched"
        );
    }

    #[test]
    fn bad_names_and_a_missing_template_are_refused() {
        let root = project("bad", &[(".env.example", EXAMPLE)]);
        for name in ["", "../x", "a b", "example"] {
            assert!(create(&root, name).is_err(), "{name:?}");
        }
        let none = project("notemplate", &[]);
        assert!(
            create(&none, "prod")
                .unwrap_err()
                .to_string()
                .contains(".env.example")
        );
    }

    #[test]
    fn variables_are_counted_without_comments_or_blank_lines() {
        let root = project(
            "count",
            &[(".env.dev", "# a comment\n\nA=1\nB=\nnot a pair\n  C = 3\n")],
        );
        assert_eq!(variable_count(&root, "dev"), 3);
        assert_eq!(variable_count(&root, "missing"), 0);
    }

    #[test]
    fn a_project_with_only_an_env_file_is_on_its_local_environment() {
        let root = project("lone", &[(".env", "A=1\n"), (".env.example", "A=1\n")]);
        assert_eq!(current(&root).as_deref(), Some("local"));
        assert!(tiers(&root).is_empty(), "there is no .env.local to list");
        // Once there are environments, `.env` is matched against them as before.
        fs::write(root.join(".env.dev"), "A=2\n").unwrap();
        assert_eq!(current(&root).as_deref(), Some("custom"));
        assert_eq!(current(&project("noenv", &[])), None);
    }

    #[test]
    fn the_first_extra_environment_keeps_the_current_env_as_local() {
        let root = project(
            "first",
            &[
                (".env", "POSTGRES_PASSWORD=mine\n"),
                (".env.example", "POSTGRES_PASSWORD=change-me\n"),
            ],
        );
        create(&root, "dev").unwrap();
        assert_eq!(
            fs::read_to_string(root.join(".env.local")).unwrap(),
            "POSTGRES_PASSWORD=mine\n",
            "what you had is now the local environment, untouched"
        );
        assert_eq!(
            fs::read_to_string(root.join(".env")).unwrap(),
            "POSTGRES_PASSWORD=mine\n"
        );
        assert_eq!(tiers(&root), ["local", "dev"]);
        assert_eq!(current(&root).as_deref(), Some("local"));
        #[cfg(unix)]
        {
            let mode = fs::metadata(root.join(".env.local"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn a_local_file_that_exists_is_never_overwritten_by_that() {
        let root = project(
            "keep",
            &[
                (".env", "A=active\n"),
                (".env.local", "A=old-local\n"),
                (".env.example", "A=1\n"),
            ],
        );
        create(&root, "dev").unwrap();
        assert_eq!(
            fs::read_to_string(root.join(".env.local")).unwrap(),
            "A=old-local\n"
        );
    }

    #[test]
    fn making_local_when_there_is_only_an_env_saves_that_one_and_makes_no_new_secrets() {
        let root = project(
            "local",
            &[
                (".env", "POSTGRES_PASSWORD=mine\n"),
                (".env.example", "POSTGRES_PASSWORD=change-me\n"),
            ],
        );
        let made = create(&root, "local").unwrap();
        assert!(made.generated.is_empty());
        assert_eq!(
            fs::read_to_string(root.join(".env.local")).unwrap(),
            "POSTGRES_PASSWORD=mine\n"
        );
    }
}
