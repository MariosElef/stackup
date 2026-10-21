//! The JetBrains run configurations (RustRover, WebStorm, IntelliJ): one Shell Script entry per part.

use super::Part;

pub(super) fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `text` as one word for a shell: in single quotes, with any single quote inside them closed,
/// escaped and opened again.
pub(super) fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// Where the launcher script sits, in the project's `.idea` folder.
pub(super) const LAUNCHER: &str = ".idea/stackup-start.sh";

/// The script the run configurations run. The IDE's Shell Script type runs a script file with an
/// interpreter (it is what mc2's configurations do, and they work): it cannot run a program
/// directly, so this one line hands over to stackup by its full path.
pub(super) fn launcher(exe: &str) -> String {
    format!(
        "#!/bin/sh\n# Made by stackup: the run configurations in .idea/runConfigurations start it.\nexec {} \"$@\"\n",
        shell_quote(exe)
    )
}

/// A JetBrains Shell Script configuration: the launcher script, run by `shell`, with the part to
/// start as its arguments. It runs in the Run window and not in a terminal tab: only there does
/// the stop button end the whole tree of processes a dev server starts. The working directory is
/// the project, so the project is `.`.
pub(super) fn config(part: Part, shell: &str) -> String {
    let options = format!("start {} --path .", part.cli());
    format!(
        r#"<component name="ProjectRunConfigurationManager">
  <configuration default="false" name="{label}" type="ShConfigurationType">
    <option name="SCRIPT_TEXT" value="" />
    <option name="INDEPENDENT_SCRIPT_PATH" value="true" />
    <option name="SCRIPT_PATH" value="$PROJECT_DIR$/{launcher}" />
    <option name="SCRIPT_OPTIONS" value="{options}" />
    <option name="INDEPENDENT_SCRIPT_WORKING_DIRECTORY" value="true" />
    <option name="SCRIPT_WORKING_DIRECTORY" value="$PROJECT_DIR$" />
    <option name="INDEPENDENT_INTERPRETER_PATH" value="true" />
    <option name="INTERPRETER_PATH" value="{shell}" />
    <option name="INTERPRETER_OPTIONS" value="" />
    <option name="EXECUTE_IN_TERMINAL" value="false" />
    <option name="EXECUTE_SCRIPT_FILE" value="true" />
    <envs />
    <method v="2" />
  </configuration>
</component>
"#,
        label = xml(&part.label()),
        launcher = LAUNCHER,
        options = xml(&options),
        shell = xml(shell),
    )
}
