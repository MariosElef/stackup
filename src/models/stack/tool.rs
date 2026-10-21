use super::choice::choice;

choice!(Tool {
    Gitleaks => "gitleaks pre-commit hook",
    Lint => "Lint & format configs",
    Ci => "ci.sh",
    Git => "git repo (main, ssh origin)",
});
