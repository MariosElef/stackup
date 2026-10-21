use super::choice::choice;

choice!(Tool {
    Makefile => "Makefile (up, dev, ci, fmt)",
    Gitleaks => "gitleaks pre-commit hook",
    Lint => "Lint & format configs",
    Ci => "ci.sh",
    Git => "git repo (main, ssh origin)",
});
