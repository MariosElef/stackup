#!/bin/sh
# Local HTTPS: one trusted certificate for localhost, api.localhost and auth.localhost.
# `mkcert -install` adds mkcert's local CA to your system trust store (it may ask for your password).
set -eu

command -v mkcert >/dev/null 2>&1 || {
    echo "mkcert is required (https://github.com/FiloSottile/mkcert#installation):" >&2
    echo "  macOS:   brew install mkcert" >&2
    echo "  Linux:   your package manager (and libnss3-tools for Firefox), or brew install mkcert" >&2
    echo "  Windows: choco install mkcert, or scoop install mkcert" >&2
    exit 1
}

mkdir -p certs
if [ ! -f certs/local.pem ]; then
    mkcert -install
    mkcert -cert-file certs/local.pem -key-file certs/local-key.pem \
        localhost 127.0.0.1 ::1 api.localhost auth.localhost
fi
