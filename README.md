<p align="center"><img src="docs/logos/header.svg" width="900" alt="stackup"></p>

<div align="center">

**Scaffold a full-stack project in one command, from the official tools.**

$\color{#00DC82}{\textbf{Nuxt}}$, $\color{#4FC08D}{\textbf{Vue}}$ or Topcoat · $\color{#DEA584}{\textbf{axum}}$ or $\color{#009688}{\textbf{FastAPI}}$

$\color{#4169E1}{\textbf{Postgres}}$ · $\color{#FF4438}{\textbf{Redis}}$ · $\color{#47A248}{\textbf{MongoDB}}$ · $\color{#00B8E3}{\textbf{Keycloak}}$ · $\color{#2496ED}{\textbf{Docker Compose}}$ · $\color{#06B6D4}{\textbf{Tailwind}}$

</div>

---

Pick a frontend (Nuxt, Vue or [Topcoat](https://github.com/tokio-rs/topcoat)), a backend (axum or
FastAPI), the services you need (Postgres, Redis, MongoDB, Keycloak, local HTTPS) and some dev
tooling. stackup runs the official scaffolders (`nuxi init`, `vp create vue`, `cargo new`,
`uv init`, `topcoat new`), adds the packages you chose, and writes the glue: Docker Compose, a CI
script, lint config, env files and a git repo. It ships no template of its own, so a project is as
current as the tools.

## Install

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/MariosElef/stackup/main/install.sh | sh

# Windows (PowerShell)
irm https://raw.githubusercontent.com/MariosElef/stackup/main/install.ps1 | iex

# anywhere, with Rust
cargo install --git https://github.com/MariosElef/stackup
```

## Use

```sh
stackup                 # the menu: Start (the wizard) and My projects
stackup new my-app      # straight to the wizard
stackup new my-app --frontend nuxt --backend axum --services postgres,redis --tools ci,git
```

**My projects** lists what you built and runs it from the menu: start the backend, the frontend or
both, `up`, logs, lazygit, environments, volumes, deploy over SSH. Every action is also a command
(`stackup up`, `stackup dev`, `stackup deploy` ...); `stackup --help` lists them.

## What you can pick

| | |
|---|---|
| Frontend | Nuxt, Vue, Topcoat, none |
| Backend | axum, FastAPI, none |
| Services | Postgres, Redis, MongoDB, Keycloak, Mailtrap, local HTTPS |
| Tools | gitleaks, lint, CI, git |

Every project has the same layers (routes, services, repositories ... on the backend; components,
services, stores ... on the frontend), and `ci.sh` fails when an import goes the wrong way.

## You need

`docker`, plus what your choices use: `pnpm`, `cargo`, `uv`, `mkcert`, `git`, `gh`. The wizard
shows what is installed and offers to install what is missing. Runs on macOS, Linux and Windows
(tested on macOS and Linux; Windows compiles and has not been run on a machine yet).

## More

The full reference, every command and how it works, is in [docs/guide.md](docs/guide.md).

```sh
cargo test              # the tests
cargo build --release
```

## License

[MIT](LICENSE). Free to use, modify and distribute, including commercially; just keep the copyright notice. Provided as is, without warranty.
