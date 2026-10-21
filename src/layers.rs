//! The layering, checked: `ui` and `commands` may use `services`, `services` may use `clients`,
//! and `models` is used by all of them but uses none. A file that imports upward fails here, so
//! the structure in the README stays true as the code grows.
#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    /// Every `.rs` file under `dir`, with its text.
    fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push((
                    path.display().to_string(),
                    fs::read_to_string(&path).unwrap(),
                ));
            }
        }
    }

    /// The files of a layer that mention a module of a layer they must not use.
    fn offenders(layer: &str, forbidden: &[&str]) -> Vec<String> {
        let mut files = vec![];
        sources(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join(layer),
            &mut files,
        );
        let mut bad = vec![];
        for (path, text) in files {
            // Test code may reach anywhere: it checks the layers working together.
            let code = text.split("#[cfg(test)]").next().unwrap_or("");
            for line in code.lines() {
                for layer in forbidden {
                    if line.contains(&format!("crate::{layer}"))
                        || line.contains(&format!("crate::{{{layer}"))
                    {
                        bad.push(format!("{path}: {}", line.trim()));
                    }
                }
            }
        }
        bad
    }

    #[test]
    fn models_use_no_other_layer() {
        let bad = offenders("models", &["ui", "commands", "services", "clients"]);
        assert!(bad.is_empty(), "models import upward:\n{}", bad.join("\n"));
    }

    #[test]
    fn clients_use_no_screens_and_no_services() {
        let bad = offenders("clients", &["ui", "commands", "services"]);
        assert!(bad.is_empty(), "clients import upward:\n{}", bad.join("\n"));
    }

    #[test]
    fn services_use_no_screens_and_no_commands() {
        let bad = offenders("services", &["ui", "commands"]);
        assert!(
            bad.is_empty(),
            "services import upward:\n{}",
            bad.join("\n")
        );
    }

    #[test]
    fn the_screens_do_not_use_the_command_line() {
        let bad = offenders("ui", &["commands"]);
        assert!(
            bad.is_empty(),
            "ui imports the commands:\n{}",
            bad.join("\n")
        );
    }
}
