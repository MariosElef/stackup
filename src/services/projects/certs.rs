//! The local HTTPS certificate: mkcert trusts its own authority once, then makes one certificate
//! for every host the project answers on. Plain steps, so it is the same on every system: no
//! shell script is needed to make it.
use std::path::Path;

use crate::models::{API_HOST, AUTH_HOST, Step};

/// Where the certificate is, inside the project.
pub const CERT: &str = "certs/local.pem";
pub const KEY: &str = "certs/local-key.pem";

/// Whether the project already has its certificate.
pub fn exists(root: &Path) -> bool {
    root.join(CERT).exists()
}

/// Trust mkcert's authority (it may ask for the computer's password), then make the certificate.
pub fn steps(root: &Path) -> Vec<Step> {
    vec![
        Step::new(
            "trust mkcert's local certificate authority",
            root,
            "mkcert",
            &["-install"],
        ),
        Step::new(
            "make the local HTTPS certificate",
            root,
            "mkcert",
            &[
                "-cert-file",
                CERT,
                "-key-file",
                KEY,
                "localhost",
                "127.0.0.1",
                "::1",
                API_HOST,
                AUTH_HOST,
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_authority_is_trusted_first_then_one_certificate_covers_every_host() {
        let steps = steps(Path::new("."));
        assert_eq!(steps[0].command_line(), "mkcert -install");
        let make = steps[1].command_line();
        assert!(
            make.starts_with("mkcert -cert-file certs/local.pem -key-file certs/local-key.pem")
        );
        for host in ["localhost", "127.0.0.1", "::1", API_HOST, AUTH_HOST] {
            assert!(make.contains(host), "{host}: {make}");
        }
    }
}
