//! Volumes: where a project's containers keep what must outlive them. This reads the compose
//! file's volumes and adds one to a service, in the file's own text, so the rest of the file
//! (comments, order, what a person wrote) is left as it is.
use std::{fs, path::Path};

use anyhow::{Result, bail};

/// One thing mounted into a service: a named volume, or a folder on this machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Mount {
    pub service: String,
    /// `pgdata` for a volume, `./data` for a folder.
    pub source: String,
    /// Where it appears inside the container.
    pub target: String,
}

impl Mount {
    /// A named volume, as opposed to a folder: folders start with `.`, `/` or `~`.
    pub fn is_named(&self) -> bool {
        !self.source.starts_with(['.', '/', '~'])
    }
}

/// A volume name: lowercase letters, digits and `_ . -`, starting with a letter or digit.
pub fn valid_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-'))
}

/// Where it appears inside a container: an absolute path, with nothing that would end the entry.
pub fn valid_mount(path: &str) -> bool {
    path.starts_with('/')
        && path.len() > 1
        && path.len() <= 200
        && !path.contains("..")
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '.' | '-'))
}

/// A folder on this machine, relative to the project: `./uploads`.
pub fn valid_folder(path: &str) -> bool {
    path.starts_with("./")
        && path.len() > 2
        && !path.contains("..")
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '.' | '-'))
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The lines of each service: `(name, first line, one past the last)`.
fn service_blocks(lines: &[&str]) -> Vec<(String, usize, usize)> {
    let Some(start) = lines.iter().position(|l| l.trim_end() == "services:") else {
        return vec![];
    };
    let end = lines[start + 1..]
        .iter()
        .position(|l| !l.trim().is_empty() && indent(l) == 0 && !l.trim_start().starts_with('#'))
        .map_or(lines.len(), |i| start + 1 + i);
    let mut heads: Vec<(String, usize)> = vec![];
    for (i, l) in lines.iter().enumerate().take(end).skip(start + 1) {
        let t = l.trim();
        if indent(l) == 2 && t.ends_with(':') && !t.starts_with(['#', '-']) {
            heads.push((t.trim_end_matches(':').to_string(), i));
        }
    }
    heads
        .iter()
        .enumerate()
        .map(|(k, (name, at))| (name.clone(), *at, heads.get(k + 1).map_or(end, |h| h.1)))
        .collect()
}

/// `a:/b`, `a:/b:ro` -> (`a`, `/b`).
fn split_entry(entry: &str) -> Option<(String, String)> {
    let entry = entry.trim().trim_matches(['"', '\'']);
    let mut parts = entry.split(':');
    let (source, target) = (parts.next()?, parts.next()?);
    (!source.is_empty() && target.starts_with('/'))
        .then(|| (source.to_string(), target.to_string()))
}

/// What each service mounts, read from `volumes: [a:/b]` and from `volumes:` with `- a:/b` lines.
pub fn mounts(text: &str) -> Vec<Mount> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = vec![];
    for (service, from, to) in service_blocks(&lines) {
        let mut i = from + 1;
        while i < to {
            let l = lines[i];
            let t = l.trim();
            if indent(l) == 4 && t.starts_with("volumes:") {
                let rest = t["volumes:".len()..].trim();
                if let Some(inner) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                    for e in inner.split(',') {
                        if let Some((source, target)) = split_entry(e) {
                            out.push(Mount {
                                service: service.clone(),
                                source,
                                target,
                            });
                        }
                    }
                } else {
                    let mut j = i + 1;
                    while j < to && (lines[j].trim().is_empty() || indent(lines[j]) >= 6) {
                        if let Some(e) = lines[j].trim().strip_prefix("- ")
                            && let Some((source, target)) = split_entry(e)
                        {
                            out.push(Mount {
                                service: service.clone(),
                                source,
                                target,
                            });
                        }
                        j += 1;
                    }
                }
            }
            i += 1;
        }
    }
    out
}

/// The names under the file's top-level `volumes:`.
pub fn declared(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines
        .iter()
        .position(|l| l.trim_end() == "volumes:" && indent(l) == 0)
    else {
        return vec![];
    };
    lines[start + 1..]
        .iter()
        .take_while(|l| l.trim().is_empty() || indent(l) > 0)
        .filter(|l| indent(l) == 2 && l.trim().ends_with(':'))
        .map(|l| l.trim().trim_end_matches(':').to_string())
        .collect()
}

/// The services in the file.
pub fn services(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    service_blocks(&lines)
        .into_iter()
        .map(|(n, _, _)| n)
        .collect()
}

/// Mount `source` at `target` in `service`. A named volume is also declared at the top of the
/// file. Refuses a service that is not there, a path that service already mounts something at,
/// and anything that is not a plain name or path.
pub fn add(text: &str, service: &str, source: &str, target: &str) -> Result<String> {
    let named = !source.starts_with('.');
    if named && !valid_name(source) {
        bail!("`{source}` is not a volume name: use lowercase letters, digits and . _ -");
    }
    if !named && !valid_folder(source) {
        bail!("`{source}` is not a folder in the project: write it like ./uploads");
    }
    if !valid_mount(target) {
        bail!("`{target}` is not a path inside a container: write it like /data");
    }
    let had_newline = text.ends_with('\n');
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let view: Vec<&str> = lines.iter().map(String::as_str).collect();
    let blocks = service_blocks(&view);
    let Some((_, from, to)) = blocks.iter().find(|(n, _, _)| n == service).cloned() else {
        bail!(
            "there is no service `{service}`: the file has {}",
            blocks
                .iter()
                .map(|b| b.0.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    };
    if mounts(text)
        .iter()
        .any(|m| m.service == service && m.target == target)
    {
        bail!("{service} already mounts something at {target}");
    }
    let entry = format!("{source}:{target}");

    // Where the service's own `volumes:` is, if it has one.
    let head =
        (from + 1..to).find(|&i| indent(&lines[i]) == 4 && lines[i].trim().starts_with("volumes:"));
    match head {
        Some(i) => {
            let t = lines[i].trim().to_string();
            let rest = t["volumes:".len()..].trim();
            if let Some(inner) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                let joined = if inner.trim().is_empty() {
                    entry.clone()
                } else {
                    format!("{}, {entry}", inner.trim())
                };
                lines[i] = format!("    volumes: [{joined}]");
            } else {
                let mut last = i;
                let mut j = i + 1;
                while j < to && (lines[j].trim().is_empty() || indent(&lines[j]) >= 6) {
                    if lines[j].trim().starts_with("- ") {
                        last = j;
                    }
                    j += 1;
                }
                lines.insert(last + 1, format!("      - {entry}"));
            }
        }
        None => {
            // At the end of the service, before the blank lines that separate it from the next.
            let mut at = to;
            while at > from + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            lines.insert(at, format!("    volumes: [{entry}]"));
        }
    }

    if named && !declared(text).iter().any(|d| d == source) {
        match lines
            .iter()
            .position(|l| l.trim_end() == "volumes:" && indent(l) == 0)
        {
            Some(top) => {
                let mut last = top;
                for (k, l) in lines.iter().enumerate().skip(top + 1) {
                    if !l.trim().is_empty() && indent(l) == 0 {
                        break;
                    }
                    if indent(l) == 2 {
                        last = k;
                    }
                }
                lines.insert(last + 1, format!("  {source}:"));
            }
            None => {
                if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                    lines.push(String::new());
                }
                lines.push("volumes:".to_string());
                lines.push(format!("  {source}:"));
            }
        }
    }
    let mut out = lines.join("\n");
    if had_newline || !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

/// The name Compose gives the project, and so the front of every volume's real name
/// (`shop_pgdata`): the folder's name, lowercased, with what Compose drops left out.
pub fn compose_project(root: &Path) -> String {
    let name: String = root
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        .collect();
    name.trim_start_matches(['_', '-']).to_string()
}

/// A volume's name as Docker has it.
pub fn docker_name(root: &Path, volume: &str) -> String {
    format!("{}_{volume}", compose_project(root))
}

/// The named volumes the file declares.
pub fn named(text: &str) -> Vec<String> {
    declared(text)
}

/// A backup file name: letters, digits and `. _ -`, ending in `.tar.gz`, nothing that is a path.
pub fn valid_backup(file: &str) -> bool {
    file.ends_with(".tar.gz")
        && file.len() > ".tar.gz".len()
        && !file.starts_with('.')
        && file
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The backups in `root/backups`, newest first (their names start with the volume and a date).
pub fn backups(root: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(root.join("backups"))
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| valid_backup(n))
                .collect()
        })
        .unwrap_or_default();
    found.sort_by(|a, b| b.cmp(a));
    found
}

/// `20261022-215900`, in UTC, for a backup's name. From seconds since 1970, with no calendar
/// library: days are counted into years the way the civil calendar does.
pub fn stamp(unix_seconds: u64) -> String {
    let days = (unix_seconds / 86_400) as i64;
    let secs = unix_seconds % 86_400;
    // Days since 1970-01-01 to a date (Howard Hinnant's civil_from_days).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// The volumes as a table: what is mounted where, and whether Docker has made it. `existing` is
/// what `docker volume ls` printed, or `None` when Docker could not be asked.
pub fn table(root: &Path, text: &str, existing: Option<&str>) -> String {
    let mut rows: Vec<[String; 4]> = vec![];
    for m in mounts(text) {
        let made = if !m.is_named() {
            "folder".to_string()
        } else {
            match existing {
                Some(list)
                    if list
                        .lines()
                        .any(|l| l.trim() == docker_name(root, &m.source)) =>
                {
                    "yes".into()
                }
                Some(_) => "not yet".into(),
                None => "?".into(),
            }
        };
        rows.push([m.source, m.service, m.target, made]);
    }
    // A volume nothing mounts yet.
    for d in declared(text) {
        if !mounts(text).iter().any(|m| m.source == d) {
            rows.push([d, "-".into(), "-".into(), "unused".into()]);
        }
    }
    if rows.is_empty() {
        return "No volumes: choose add volume to store something.\n".to_string();
    }
    let head = ["volume", "service", "mounted at", "created"];
    let widths: Vec<usize> = (0..4)
        .map(|c| {
            rows.iter()
                .map(|r| r[c].chars().count())
                .chain([head[c].len()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |cells: [&str; 4]| {
        cells
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{c:<w$}", w = widths[i]))
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_string()
    };
    let mut out = line(head) + "\n";
    for r in &rows {
        out += &line([&r[0], &r[1], &r[2], &r[3]]);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENERATED: &str = "services:\n  postgres:\n    image: postgres:alpine\n    ports: [\"5432:5432\"]\n    volumes: [pgdata:/var/lib/postgresql]\n    healthcheck:\n      retries: 20\n\n  redis:\n    image: redis:alpine\n    ports: [\"6379:6379\"]\n\n  keycloak:\n    image: kc\n    volumes:\n      - ./keycloak:/opt/keycloak/data/import:ro\n\n  backend:\n    image: shop-backend\n\nvolumes:\n  pgdata:\n";

    #[test]
    fn mounts_are_read_from_a_list_in_brackets_and_from_dashes() {
        let all = mounts(GENERATED);
        assert_eq!(all.len(), 2, "{all:?}");
        assert_eq!(
            all[0],
            Mount {
                service: "postgres".into(),
                source: "pgdata".into(),
                target: "/var/lib/postgresql".into()
            }
        );
        assert_eq!(all[1].service, "keycloak");
        assert!(
            all[0].is_named() && !all[1].is_named(),
            "a folder is not a named volume"
        );
        assert_eq!(declared(GENERATED), ["pgdata"]);
        assert_eq!(
            services(GENERATED),
            ["postgres", "redis", "keycloak", "backend"]
        );
    }

    #[test]
    fn a_volume_is_added_to_a_service_that_has_none_and_declared_at_the_top() {
        let out = add(GENERATED, "redis", "redisdata", "/data").unwrap();
        assert!(out.contains("  redis:\n    image: redis:alpine\n    ports: [\"6379:6379\"]\n    volumes: [redisdata:/data]\n"), "{out}");
        assert!(
            out.ends_with("volumes:\n  pgdata:\n  redisdata:\n"),
            "{out}"
        );
        // Everything else is as it was.
        assert!(out.contains("healthcheck:\n      retries: 20"));
        assert_eq!(mounts(&out).len(), 3);
        assert_eq!(declared(&out), ["pgdata", "redisdata"]);
    }

    #[test]
    fn a_volume_joins_an_existing_list_in_either_style() {
        let out = add(GENERATED, "postgres", "backups", "/backups").unwrap();
        assert!(
            out.contains("volumes: [pgdata:/var/lib/postgresql, backups:/backups]"),
            "{out}"
        );
        let out = add(GENERATED, "keycloak", "kcdata", "/opt/data").unwrap();
        assert!(
            out.contains(
                "      - ./keycloak:/opt/keycloak/data/import:ro\n      - kcdata:/opt/data\n"
            ),
            "{out}"
        );
    }

    #[test]
    fn a_folder_is_mounted_without_being_declared() {
        let out = add(GENERATED, "backend", "./uploads", "/uploads").unwrap();
        assert!(
            out.contains(
                "  backend:\n    image: shop-backend\n    volumes: [./uploads:/uploads]\n"
            ),
            "{out}"
        );
        assert_eq!(declared(&out), ["pgdata"], "a folder needs no declaration");
        let m = mounts(&out);
        assert!(m.iter().any(|m| m.service == "backend" && !m.is_named()));
    }

    #[test]
    fn a_file_with_no_volumes_section_gets_one() {
        let text = "services:\n  app:\n    image: x\n";
        let out = add(text, "app", "data", "/data").unwrap();
        assert_eq!(
            out,
            "services:\n  app:\n    image: x\n    volumes: [data:/data]\n\nvolumes:\n  data:\n"
        );
    }

    #[test]
    fn what_cannot_be_added_is_refused_with_the_reason() {
        let err = |r: Result<String>| r.unwrap_err().to_string();
        assert!(err(add(GENERATED, "nope", "a", "/a")).contains("no service `nope`"));
        assert!(err(add(GENERATED, "redis", "Bad Name", "/a")).contains("not a volume name"));
        assert!(
            err(add(GENERATED, "redis", "data", "relative"))
                .contains("not a path inside a container")
        );
        assert!(err(add(GENERATED, "redis", "data", "/a/../b")).contains("not a path"));
        assert!(err(add(GENERATED, "redis", "./a/../b", "/a")).contains("not a folder"));
        assert!(
            err(add(GENERATED, "postgres", "other", "/var/lib/postgresql"))
                .contains("already mounts")
        );
        // Nothing that could end the entry or start another key.
        assert!(!valid_name("a:b") && !valid_name("a b") && !valid_name("-a") && !valid_name(""));
        assert!(!valid_mount("/a:b") && !valid_mount("/a b") && !valid_mount("/"));
    }

    #[test]
    fn the_project_and_volume_names_are_what_compose_makes() {
        assert_eq!(compose_project(Path::new("/work/My Shop_2")), "myshop_2");
        assert_eq!(compose_project(Path::new("/work/-odd")), "odd");
        assert_eq!(
            docker_name(Path::new("/work/shop"), "pgdata"),
            "shop_pgdata"
        );
    }

    #[test]
    fn a_backup_is_a_plain_file_name_ending_in_tar_gz() {
        assert!(valid_backup("pgdata-20261022-215900.tar.gz"));
        for bad in [
            "x.zip",
            ".tar.gz",
            "../x.tar.gz",
            "a/b.tar.gz",
            "a b.tar.gz",
            ".hidden.tar.gz",
        ] {
            assert!(!valid_backup(bad), "{bad}");
        }
    }

    #[test]
    fn the_timestamp_is_the_right_date_and_time_in_utc() {
        assert_eq!(stamp(0), "19700101-000000");
        assert_eq!(stamp(1_000_000_000), "20010909-014640");
        assert_eq!(
            stamp(1_709_164_800 + 3 * 3600 + 5 * 60 + 9),
            "20240229-030509",
            "a leap day"
        );
        assert_eq!(stamp(1_792_706_340), "20261022-215900");
    }

    #[test]
    fn backups_are_listed_newest_first_and_only_real_ones() {
        let root = std::env::temp_dir().join(format!("stackup-backups-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("backups")).unwrap();
        for f in [
            "pg-20260101-000000.tar.gz",
            "pg-20261022-215900.tar.gz",
            "notes.txt",
        ] {
            fs::write(root.join("backups").join(f), "x").unwrap();
        }
        assert_eq!(
            backups(&root),
            ["pg-20261022-215900.tar.gz", "pg-20260101-000000.tar.gz"]
        );
        assert!(backups(&root.join("nowhere")).is_empty());
    }

    #[test]
    fn the_table_says_what_is_mounted_where_and_whether_docker_made_it() {
        let root = Path::new("/work/shop");
        let t = table(root, GENERATED, Some("shop_pgdata\nother_thing\n"));
        assert!(t.starts_with("volume"), "{t}");
        assert!(
            t.contains("pgdata")
                && t.contains("postgres")
                && t.contains("/var/lib/postgresql")
                && t.contains("yes"),
            "{t}"
        );
        assert!(t.contains("./keycloak") && t.contains("folder"), "{t}");
        assert!(table(root, GENERATED, Some("")).contains("not yet"));
        assert!(table(root, GENERATED, None).contains('?'));
        assert!(table(root, "services:\n  a:\n    image: x\n", Some("")).contains("No volumes"));
    }
}
