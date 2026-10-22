<div align="center">

# stackup guide

The full reference. The short version is in the [README](../README.md).

**Scaffold a full-stack project in one command, from the official tools.**

$\color{#00DC82}{\textbf{Nuxt}}$, $\color{#4FC08D}{\textbf{Vue}}$ or Topcoat · $\color{#DEA584}{\textbf{axum}}$ or $\color{#009688}{\textbf{FastAPI}}$

$\color{#4169E1}{\textbf{Postgres}}$ · $\color{#FF4438}{\textbf{Redis}}$ · $\color{#47A248}{\textbf{MongoDB}}$ · $\color{#00B8E3}{\textbf{Keycloak}}$ · $\color{#2496ED}{\textbf{Docker Compose}}$ · $\color{#06B6D4}{\textbf{Tailwind}}$

</div>

---

`stackup` is a small Rust CLI with a full-screen menu. **Start** opens a wizard: you pick a frontend,
a backend, the services you need and some dev tooling. It then runs the *official* scaffolders
(`cargo new`, `uv init`, `nuxi init`, Vite+'s `vp create vue`), adds the packages for what you
chose, and writes the glue around them: Docker Compose, a CI script, lint and format config, env
files, and a git repo. **My projects** lists everything you built with stackup, and runs, builds
and deploys it from the same menu. Creating a project runs inside the menu too, in its output panel,
and ends with the new project selected on your list. There is no separate CLI to install in the project.
Selecting a project shows it at a glance: its stack with each technology's logo, whether it is
running, its services, its git history and the room it takes (see [The project list](#the-project-list)).

**New: [Topcoat](https://github.com/tokio-rs/topcoat) is now a frontend option**, next to Nuxt and
Vue. It is a Rust framework that renders on the server, so a Rust-only stack is possible
(Topcoat + axum). stackup scaffolds it with `topcoat new`, gives it Tailwind, Lucide icons and
i18n (`rust-i18n`) when you tick them, its own layers (`app`, `features`, `components`,
`composables`, `services`, ...) with the dependency direction checked in `ci.sh`, and a Dockerfile.
See [Topcoat](#topcoat) below.

It doesn't ship its own copy of a framework template. Every project starts from what the
official tool generates (`nuxi init`, `vp create vue`, `cargo new`, `uv init`, `topcoat new`), so it
is as current as the tool you have, and the packages are added at their latest versions. stackup
only adds the glue around it. The one tool it does not take as it finds it is Topcoat's CLI, which
changes quickly: stackup installs a revision it has been tested with, unless you pick the latest.
Every version can be chosen when you press Create (see [Versions](#versions)).

```
┌ stackup · my-app ──────────────┐┌ files │ docker-compose.yml ───────────┐
│ Frontend                       ││ my-app/                               │
│ ( ) Nuxt                       ││ ├─ frontend/                          │
│ (•) Vue (Vite)                 ││ │  ├─ src/                            │
│ ( ) none                       ││ │  │  ├─ main.ts                      │
│                                ││ │  │  ├─ stores/                      │
│ Backend                        ││ │  │  └─ services/                    │
│ (•) axum (Rust)                ││ │  └─ Dockerfile                      │
│ ( ) FastAPI (Python)           ││ ├─ backend/                           │
│                                ││ │  ├─ src/{routes,services,…}/        │
│ Services                       ││ │  └─ Cargo.toml                      │
│ [x] Postgres                   ││ ├─ docker-compose.yml                 │
│ [x] Redis                      ││ ├─ docker-compose.yml                 │
│ ▶ Create project               ││ └─ ci.sh                              │
└────────────────────────────────┘└───────────────────────────────────────┘
 NORMAL  menu   needs: pnpm ✓  cargo ✓  git ✓
```

## Install

stackup runs on **macOS, Linux and Windows**. Pick one:

```sh
# macOS / Linux: the latest release, with curl or wget
curl -fsSL https://raw.githubusercontent.com/MariosElef/stackup/main/install.sh | sh
wget -qO- https://raw.githubusercontent.com/MariosElef/stackup/main/install.sh | sh
```

```powershell
# Windows, in PowerShell
irm https://raw.githubusercontent.com/MariosElef/stackup/main/install.ps1 | iex
```

```sh
# From source, on any system (needs Rust)
cargo install --git https://github.com/MariosElef/stackup
```

or from a clone: `git clone https://github.com/MariosElef/stackup && cd stackup && cargo install
--path .`. The `packaging/` folder has what a package manager needs: a Homebrew formula, an Arch
`PKGBUILD`, a `.deb` (`cargo deb`), and Scoop and winget manifests. The release scripts need a
tagged release to exist (`git tag v0.1.0 && git push --tags` builds one with GitHub Actions).

### On Windows

stackup needs **Git for Windows** (its `sh`, `cp` and `grep` run the generated `ci.sh` and a few
steps), **Docker Desktop** (running), and, depending on what you pick, `pnpm`, Rust (`rustup`),
`uv`, `mkcert`, `gh` and `lazygit`. Create offers to install what is missing with `winget`
(Git for Windows included: one button), and shows the download for the rest (Rust and Docker
Desktop). A step that needs Git's shell when it is missing says so, with the command and the
download link, and `stackup doctor` checks for it. The list of projects is kept in `%APPDATA%\stackup`, SSH
keys go in `%USERPROFILE%\.ssh`, and the IDE buttons call PowerShell. Line endings of the scripts
in a generated project stay LF (it ships a `.gitattributes`), so they still run in Docker and CI.
It is developed and tested on macOS. The whole test suite also passes on Linux, and the code
compiles for Windows (`cargo check --target x86_64-pc-windows-msvc`), but it has not been run on a
Windows machine yet: tell me what breaks. CI runs the tests on all three.

## Use

```sh
stackup                     # the menu: Start, My projects
stackup new my-app          # straight to the wizard (and it builds in this terminal, for scripts)
stackup new .               # scaffold into the current (empty) folder

# no wizard: give every choice as a flag
stackup new my-app --frontend nuxt --backend axum --libs pinia,colada,i18n \
  --services postgres,redis --tools gitleaks,lint,ci,git
```

A name keeps the case you type: `Portfolio` is the folder, the entry in My projects, the README
title and the GitHub repository. What must be lowercase (crates, Docker images, the Compose
project) uses a lowercase form of it, so `Portfolio` builds `portfolio-backend`.

Flags you pass pre-fill the wizard. Give `--frontend`, `--backend`, `--services` and `--tools`, and
it is skipped. Then:

```sh
cd my-app
stackup up                 # docker compose up --build (makes the HTTPS certificates first if needed)
stackup                    # or the menu: My projects -> my-app -> up
```

### Requirements

On your `PATH`, depending on what you pick: `pnpm` (Nuxt and Vue), `cargo` (axum and Topcoat), the
Topcoat CLI (Topcoat), `uv` (FastAPI), `git`, `docker`, `mkcert` (local HTTPS), and `gh` (the
GitHub repository, with the git tool ticked). `lazygit` is optional: the menu's git client. Docker is Docker Desktop on a Mac or Windows
machine (it has to be running), and Docker Engine on Linux. The wizard's footer shows what is
installed, and Create offers to install what is missing (see [The wizard](#the-wizard)).

Topcoat's `topcoat new` is not in the crates.io release of its CLI yet, so install it from git
(needs a recent Rust toolchain):

```sh
cargo install --git https://github.com/tokio-rs/topcoat topcoat-cli
```

## What you can pick

| Choice | Options |
|---|---|
| Frontend | `nuxt`, `vue`, `topcoat`, `none` |
| Backend | `axum`, `fastapi`, `none` |
| Libraries | `pinia`, `colada`, `i18n`, `zod`, `openapi`, `lucide`, `tailwind`, `vueuse` (`--libs`). Topcoat uses `tailwind`, `lucide` and `i18n`; the rest are dropped with a note. |
| Backend libraries | `utoipa` (`--backend-libs`), axum only: serves the OpenAPI document at `/openapi.json`. FastAPI already does. |
| Services | `postgres`, `redis`, `mongodb`, `keycloak`, `mailtrap`, `https` |
| Tools | `gitleaks`, `lint`, `ci`, `git` |

Pinia Colada is built on Pinia, so ticking it adds Pinia for you.

## What you get

Every project is layered the same way, so you always know where code goes. Empty folders hold a
`.gitkeep` (Python packages get an `__init__.py`).

```
backend (axum)     src/{routes,services,repositories,models,clients,middleware}/
backend (FastAPI)  app/{routes,services,repositories,models,schemas,clients,auth}/  + tests/
frontend (Nuxt)    app/{pages,components,services,stores,composables,…}/
frontend (Vue)     src/{views,components,services,stores,composables,…}/
frontend (Topcoat) src/{app,features,components}  from `topcoat new`, plus {composables,services,middleware,types,utils}/
```

The layering is **enforced**, not just suggested. A violation fails `ci.sh`:

- **axum:** routes and services can't touch `sqlx`, `redis` or `mongodb`; routes can't use `clients`.
- **FastAPI:** `import-linter` contracts: routes → services → repositories, and nothing below the
  routes imports `fastapi`.
- **Topcoat:** `cargo fmt` and clippy with `-D warnings`, plus grep checks on `crate::` imports:
  `app` → `features` → `components` → `composables` → `services` → `types`/`utils`; components can't
  import features or services, composables can't import components, and services, middleware, types
  and utils can't import anything above them. Composables are plain functions that take Topcoat's
  `cx` (for example `fn api(cx: &Cx) -> &ApiClient`). Topcoat's generated README states
  the first part of this rule; stackup enforces it.
- **Frontend (Nuxt, Vue):** oxlint `no-restricted-imports`: components can't import stores or services, and
  services can't import stores or UI.

### Topcoat

[Topcoat](https://github.com/tokio-rs/topcoat) is a Rust framework that renders pages on the
server. In stackup it is only the frontend: it sits in `frontend/` beside the axum or FastAPI
backend, has no database of its own, and reaches the backend through `API_URL`. Pick **Topcoat
(Rust)** in the wizard, or pass `--frontend topcoat`.

- **Scaffolded by `topcoat new`**, so it starts from what Topcoat generates today.
- **Libraries:** Tailwind and Lucide map to Topcoat's own features (Iconify's `lucide` set) and its
  UI components come with both. i18n adds `rust-i18n`, `locales/en.yml` and the `i18n!` call in
  `main.rs`. The npm libraries (Pinia, Zod, VueUse...) don't apply and are dropped with a note.
- **Layers:** `app`, `features` and `components` from Topcoat, plus `composables`, `services`,
  `middleware`, `types` and `utils`, with the dependency direction checked in `ci.sh` (below).
- **Docker:** a Dockerfile in axum's pattern, which also bundles the assets the release binary needs.
- **Needs:** `cargo` and the Topcoat CLI. Its `topcoat new` is not in the crates.io release yet, so
  install it from git (see Requirements). Topcoat is version 0.10, so expect breaking changes.

### Libraries bring their own folders and wiring

| Library | Packages | Adds |
|---|---|---|
| Pinia | `pinia` (+ `@pinia/nuxt`) | `stores/` |
| Pinia Colada | `@pinia/colada` (+ `@pinia/colada-nuxt`) | `queries/` |
| i18n | `@nuxtjs/i18n` (Nuxt), `vue-i18n` (Vue), `rust-i18n` (Topcoat) | Topcoat: `locales/en.yml`, `i18n!` in `main.rs`. Nuxt: `i18n/locales/en.json` and module config. Vue: `src/i18n.ts`, `src/locales/en.json`, wired in `main.ts` |
| Zod | `zod` | `contracts/` |
| OpenAPI client | `@hey-api/openapi-ts` | `contracts/`, `openapi-ts.config.ts`, `pnpm generate:api`. With axum and `utoipa`, it points at the spec the backend serves |
| Lucide, VueUse | `@lucide/vue`, `@vueuse/core` | Topcoat: Lucide only, as Iconify's `lucide` set built by `build.rs` |
| Tailwind CSS 4 | `tailwindcss`, `@tailwindcss/vite` | Vite plugin and `@import "tailwindcss"` |

### Lint, format and types

| | Nuxt | Vue | axum | FastAPI |
|---|---|---|---|---|
| Format | oxfmt | Vite+ (`vp check`) | `cargo fmt` | ruff |
| Lint | oxlint + ESLint (Vue template rules) | Vite+ | clippy, cargo-deny | ruff, import-linter |
| Types | `vue-tsc` | Vite+ | the compiler | pyrefly |

### Services

- **Postgres, Redis, MongoDB** run in Docker Compose. Compose contains exactly the services you
  picked.
- **Keycloak** runs in Docker (`start-dev`) and imports `keycloak/app-realm.json` on every start:
  realm `app`, a public PKCE client `app-frontend`, a bearer-only `app-backend`, and the user
  `demo` / `demo`. The backend gets `OIDC_ISSUER_URL` and `OIDC_JWKS_URL`. There is no data volume:
  the realm file is the source of truth.
- **Mailtrap** is a hosted email sandbox, so there is no container. `MAILTRAP_*` settings go into
  the env files and reach the backend as `SMTP_*`.
- **Local HTTPS** puts Caddy in front of everything with one mkcert certificate:
  `https://localhost:8443` (app), `https://api.localhost:8443`, `https://auth.localhost:8443`.
  The certificate is made when the project is created (and again by `stackup up` or `stackup
  certs` when it is missing) with two plain `mkcert` commands, so it is the same on every system;
  `scripts/certs.sh` is for people who use the project without stackup. It uses 8443 because
  Docker Desktop's Kubernetes commonly holds 80 and 443.

### Environments

A new project has two files: **`.env`**, with the real values (gitignored; it is what Docker
Compose and `stackup dev` read), and **`.env.example`**, the committed template. That is one
environment, the local one. When you make another (`.env.dev`, `.env.prod`, `.env.staging`), the
current `.env` is first saved as `.env.local`, untouched, so nothing is lost; from then on `.env`
is a copy of the active one, and after switching you run `up` again. All of them are gitignored
except `.env.example`.

- **See and switch:** a project's page shows the active environment in the title, in its color
  (local green, dev yellow, prod red). `e` opens a picker of the environments the project has, the
  active one marked; Enter switches. `stackup env` lists them, `stackup env prod` switches.
- **Make one:** the last choice in the picker is **+ a new environment**, or `stackup env prod
  --create`, or `shop · new environment` in the command palette. It copies `.env.example` and gives
  every secret (`*PASSWORD*`, `*SECRET*`, `*TOKEN*`) a new random value, including the copy of the
  password inside a connection URL. The file is readable only by you. It then tells you which
  values it left as the template had them (an address that still says localhost, or an empty
  value) so you can set them. A tier that exists is never overwritten.
- **`dev` loads the active `.env`** into the backend it runs on your machine; in Docker, Compose
  passes the values in.
- **Deploy** sends `.env.prod` to the server as its `.env`, whichever tier is active locally.

### The menu and the project commands

Run `stackup` with no command for the menu, laid out like mc2's: a home
screen with the logo, **My projects**, then a project's actions on the left, and what an action
does (or, once it runs, its output on an in-app terminal) on the right. `j`/`k` move, `<CR>` opens
or runs, `h` goes back, `?` lists the keys. While an action runs every key goes to it, so prompts
and `ssh`'s questions work; `Ctrl-]` comes back to the menu and stops it.

### The project list

**My projects** shows the project selected on the left and, on the right, panels about it:

- **State.** The card's frame and title are green when every service with a port answers on it,
  yellow when some do, gray when none, red when the folder is gone. This asks the ports, not Docker:
  another program on the same port counts as up.
- **Built with.** Two cards side by side, *frontend* and *backend*, each with the technology's
  real logo and its name in block letters: Nuxt, Vue, Topcoat (its own wordmark), FastAPI, and
  Ferris the crab for axum. A project without a frontend or a backend gets a card that says so.
  The logos are pixel bitmaps made from the projects' official artwork (Vue, Nuxt and FastAPI from
  simple-icons, CC0; Topcoat's from its repository; Ferris from rustacean.net, CC0) and drawn in
  half-block characters, not images, so every terminal shows them, in each brand's colors with a
  glint that sweeps across. They belong to their projects. The cards only appear when the
  terminal is tall enough (about 40 rows); the panels below matter more.
- **Inside a project.** The bottom of the project's page has a panel for each thing it has: *open in a browser*, *connect to* (its data) and *built with*, which holds the same logos, small (four rows).
- **Git.** The branch, what changed, how far ahead or behind, the last commits and the remote.
- **Services.** Every container in the compose file, with its port and whether it answers.
- **Room it takes.** How much space the project, its build files, `node_modules` and `.git` use,
  measured in the background so moving between projects never waits for a big `target/`.
- **Keys.** The keys that work here.

**Command palette.** `:` or `Ctrl-P` opens a box with every command: start, add, jump to any
project (type part of its name), and every action that applies to the selected project
(`shop · up`, `shop · deploy`, `shop · open in the browser`...). Type to filter, with scattered
letters allowed and the closest match first; the arrows move, Enter runs, Esc closes.

**Where it is.** A project's page shows where it can be reached, read from its compose file: the
web addresses (with `https://…:8443` hostnames behind the local HTTPS proxy), a dot for whether
something is answering on each right now, and where to connect to its databases. It stays under
the output while `up` runs. `o` opens the app in your browser. `stackup urls` lists the addresses
and `stackup open` opens the app.

Projects are tracked in a list file (see below). `a` adds projects you already have: it asks for a
folder, and finds every project in it or two folders below (anything with a `docker-compose.yml`,
skipping `node_modules`, `target` and hidden folders). A project already on the list is left as it
was. `stackup add [PATH]` does the same from the command line. `r` **removes** a project: it stops
its containers and deletes their data, deletes the project's folder, and drops it from the list.
That cannot be undone, so the project's name is typed back first, and it warns if the folder has
uncommitted changes. It refuses a folder that holds your home folder, one close to the top of the
disk, or the one you are standing in. When the project has a repository on GitHub or GitLab (read from its `origin`) and `gh` or
`glab` is installed, `r` first asks where to remove it from: this computer only, or this computer
and the repository there too. Deleting the repository goes first, so if it fails nothing local has
been deleted; GitHub is asked for the `delete_repo` permission once. `stackup remove --github` or
`--gitlab` does the same from the command line. Deleting a repository cannot be undone. When it is done, the project's name is shown in big letters that crumble away (under two seconds; any key skips it) and then you are back on the list. `d` only stops listing a project and never touches its files.

Every project you make with stackup is remembered in `~/.config/stackup/projects.json`
(`$STACKUP_HOME` moves it), so it shows up under **My projects**. `d` stops listing one; its files
are never touched.

The same actions are commands, run from inside a project, or with `-p NAME` or `--path DIR`:

```
up [-d]        Build and start everything in Docker (makes the HTTPS certificates if missing)
down  ps  logs [SERVICE]
stop           Stop the containers and keep them stopped, also after a restart
images  clean  List the images the project built; delete its containers and those images
volumes        The volumes and which service mounts each; `volume add|backup|restore` below
cargo-clean    Delete the Rust build files (target/), which grow with every build; shows how much it frees
dev            Start the databases in Docker and run the backend and the frontend with reload
migrate        Apply the migrations not applied yet (sqlx for axum, Alembic for FastAPI)
migration new NAME     Make the next migration in backend/migrations
migration revert       Undo the last migration
start PART     Run backend, frontend or fullstack for development (what the IDE buttons call)
ide            Write the IDE run configurations (Start backend / frontend / fullstack) and the VS Code
               extensions; `--install-extensions` installs those with the `code` command
git open|init|repo   Open lazygit; start a repository; create the GitHub repository (--public)
build          Build the images, for this machine or --platform linux/amd64
deploy --host HOST   Build for linux, send the images over SSH, copy the compose file and .env, start them
ci  fmt        The checks CI runs; format the backend and frontend
env [TIER]     List environments, or switch (`--create` makes one from .env.example)
certs  hooks   Local HTTPS certificates; the gitleaks hook
doctor         Can this machine run it? One line per check, each failure names its fix
urls  open     Where the project is reachable; open its app in the browser
add [PATH]     Add projects that already exist (a project folder, or a folder of them)
remove         Delete a project: its containers, their data and its folder (asks first; --yes skips)
projects       List the projects you built
```

`stackup --help` is the authority on exact syntax. Every action takes `--dry-run`, which prints the
steps without running any.

`deploy` needs SSH access to a server with Docker. It builds for `linux/amd64`, streams the images
with `docker save | ssh docker load` (no registry), copies `docker-compose.yml` and `.env.<tier>`
(`prod` by default, sent as `.env`) and any mounted folders, and runs `docker compose up -d`.
Compose gives the images explicit names (`<project>-backend`, `<project>-frontend`) so it knows
what to ship. The host is remembered for next time, and before anything is sent the project's name
has to be typed back (`--yes` skips that for scripts). The command line uses the menu's colors,
and switches them off when piped or when `NO_COLOR` is set.

### Volumes: storing things

A container forgets what it wrote when it is rebuilt, unless something outlives it. In a project's
page, **Docker → volumes** lists what the compose file mounts (the name or folder, the service,
the path inside the container, and whether Docker has made it yet), and three actions manage them:

- **add volume** asks for a name (or a `./folder`), picks the service from the compose file's own,
  and asks for the path inside the container (`/data` is filled in). A name makes a Docker volume
  that Docker keeps; a `./folder` shares a folder of the project, made first so Docker does not
  make it as root. stackup writes the entry into `docker-compose.yml` (and declares the volume at
  the top), leaving the rest of the file as it was, then starts that service again with it.
- **backup volume** packs a volume into `backups/<volume>-<date>.tar.gz` with a throwaway
  container, so it works for a database too, and nothing is stopped. `backups/` is gitignored.
- **restore volume** puts a backup back, replacing what is in the volume. The running services
  that use it are stopped first and started again after, and the project's name is typed back.

From the command line: `stackup volumes`, `stackup volume add NAME --service SERVICE --mount
/data`, `stackup volume backup NAME`, `stackup volume restore NAME FILE [--yes]`. A name is
lowercase letters, digits and `. _ -`; paths are plain. The volumes' real names are the project
folder's name, lowercased, then an underscore (`shop_pgdata`).

### Git

**Git is lazygit.** On a project, choose *lazygit* (or find it in the `:` palette): the real
[lazygit](https://github.com/jesseduffield/lazygit) opens on the project, inside the app's own
pane and using the whole page, not in a new terminal. Stage, commit, switch branches, push and
pull the way you do in lazygit; quit it with `q` and you are back on the project, with its branch
and changes read again. It needs lazygit installed (`brew install lazygit`); without it the menu
says how to get it. The project's action list is in sections (Run, Docker, Build & ship,
Environment, Git, Project): `{` and `}` jump between them. Under Git there are also *init* (a folder that
is not a repository yet) and *create repo* (it needs `gh`); lazygit does the rest, so there is no
separate commit, push, pull or branch command. `stackup git open|init|repo` does the same from the
command line.

**Setting up GitHub.** With the git tool ticked, Create needs the GitHub CLI (`gh`, installed
with brew if you say so). If `gh` is not signed in, or there is no SSH key, a **Set up GitHub**
window lists exactly what it will run, and Set up runs it on the in-app terminal: make
`~/.ssh/id_ed25519` (no passphrase), `gh auth login` with SSH (it opens your browser once),
upload the key to your GitHub account, add a `Host github.com` entry to `~/.ssh/config` so every
program here uses the key, and check that GitHub lets you in. Then you press Create again.
Afterwards `git push` works from any terminal on the computer, with nothing more to do. Skip makes
the project anyway. Steps already done are left out, and a `Host github.com` entry of your own is
never touched.

### IDEs

Creating a project (or `stackup ide` in an existing one) sets up both IDEs for what it has.

- **The launch banner.** Whatever starts a part (the play buttons, `stackup dev`, the menu) first
  prints a header: each running part with its small logo, its name, where it answers on this
  machine (`http://localhost:8000`), the HTTPS address when the project has the proxy (those
  answer when the whole stack runs in Docker with `stackup up`, not for the servers started
  here), what it runs and in which folder, and the databases it uses. Everything the compilers
  and watchers print follows under it.
- **Play buttons.** *Start backend*, *Start frontend* and, with both, *Start fullstack*. In
  JetBrains (RustRover, WebStorm...) they are in the run dropdown: press ▶, and ■ stops both servers.
  They are Shell Script run configurations that run `.idea/stackup-start.sh`, a one-line launcher
  that hands over to stackup by its full path (the IDE runs script files, not programs, which is also
  how mc2's work). In VS Code they are in Run and Debug (the ▶ next to the dropdown) and, as
  tasks, in *Terminal → Run Task*. They hold no values and are gitignored, since they name your
  stackup's path: someone who clones the project runs `stackup ide` once.
- **Language servers.** VS Code has none for Rust or Vue, so the project asks for the right
  extensions in `.vscode/extensions.json` (committed, so the whole team is offered them): for Rust
  `rust-analyzer` and Even Better TOML; for Nuxt and Vue Vue (Official, the Volar server), Oxc and
  ESLint; for FastAPI Python, Pylance and Ruff; Docker and YAML for every project; Caddyfile Support
  with HTTPS. `.vscode/settings.json` turns on clippy and format-on-save for Rust and Ruff for
  Python. `stackup ide --install-extensions` installs the ones you do not have, with the `code`
  command. A `tasks.json`, `launch.json`, `extensions.json` or `settings.json` of your own is never
  overwritten.
- **One workspace.** A Topcoat frontend with an axum backend is one Cargo workspace: one
  `Cargo.lock`, one toolchain, one `Dockerfile` that builds both.

**Local HTTPS.** With the HTTPS service ticked, the certificate is made while the project is
created (Create offers to install mkcert if it is missing; your OS asks for a password once to
trust it) and Caddy serves everything over HTTPS from the first `up`.

**HTTPS while developing.** The proxy above is a container that forwards to the other containers,
but a ▶ start or `stackup dev` runs the backend and frontend on your machine. So the project has a
second proxy, `caddy-dev`, in its `docker-compose.yml` under the `dev` profile (`up` never starts
it) and a `Caddyfile.dev` that forwards the same `https://app.localhost:8443` and
`https://api.localhost:8443` to your machine with `host.docker.internal`. Starting a part also
starts it, so those addresses work while you develop, and the launch banner shows them as links
only when something answers. A project made before this gets both files the first time it is
started (`stackup internal https-dev` does it). It is left alone when something already answers
on 8443, as the whole stack running with `stackup up` does. It needs the certificate, so a project
without local HTTPS has none of this. Docker Desktop (Mac, Windows) knows `host.docker.internal`;
on Linux the service maps it with `host-gateway` (Docker 20.10 or newer), and a Nuxt dev server is
started with `HOST=0.0.0.0` there so the container can reach it. Vue and Topcoat on Linux, and
Windows as a whole, are not tested.

With the `git` tool ticked (the default), stackup runs `git init -b main` and adds an `origin`
remote built from your `gh` login or `git config github.user`: SSH when `gh` or a key in `~/.ssh`
says so, https otherwise. If it can't work out a username, it adds no remote and tells you.

| Flag | Effect |
|---|---|
| `--remote <url>` | Use this origin instead |
| `--create-remote` | Make the first commit, create a private repo with `gh`, and push |
| `--create-remote --public` | Same, but public |
| `--no-git` | Skip git entirely |

Nothing in the git step can fail the scaffold: if `git`, `gh` or the network isn't ready, you get
a note and the project is still written.

The generated apps only expose `/health`. Add your own routes.

## The wizard

Pressing **Create project** checks that the tools your stack needs are installed (`cargo`, `uv`,
`pnpm`, the Topcoat CLI, `mkcert`, `git`, `gh` with the git tool, and `docker` for every project). If one is missing, a notice opens over the wizard:
what is missing, what it is for, and the exact command that **Install** would run, with **Install**
and **Close** buttons. So the Topcoat CLI is only ever installed when you are making a Topcoat
project, and only if you say so. Install runs on the in-app terminal; when it ends you are back in
the wizard to press Create again. Before it runs, Install asks **which version** where there is a
real choice: which Rust (the ones already installed first, then latest stable, then common ones),
which Topcoat CLI (the revision stackup is tested with, or the latest), and which pnpm. One question
at a time, the recommended option first, with the arrows or a number to pick and Esc to go back.
Rust and Docker are never installed from here, and git only on Windows (with `winget`, as Git for
Windows): the notice says how to get them (Docker Desktop on a Mac or Windows machine, Docker
Engine on Linux). `stackup doctor` checks the same tools for an existing project.

Two panes: the menu on the left and a live preview on the right (the file tree that will be
written, or the generated `docker-compose.yml`). The focused pane has the bright border. Keys
follow LazyVim.

| Key | Action |
|---|---|
| `Ctrl-h` / `Ctrl-l` (or `h` / `l`) | focus the menu / the preview |
| `j` `k` (or `↓` `↑`) | move in the menu, scroll in the preview; `5j` moves five |
| `gg` / `G` | top / bottom |
| `Ctrl-d` / `Ctrl-u` | half a page |
| `{` / `}` | previous / next section |
| `Space` / `Enter` / `x` | pick a radio option, tick a checkbox, or create on the last row |
| `H` / `L` / `Tab` | switch the preview tab: files / docker-compose.yml |
| `/` then `n` / `N` | search the menu, next, previous |
| `:q` / `:wq` | quit / create |
| `?` | keymap help |
| `q` / `Ctrl-c` | quit without creating anything |

### Versions

Pressing Create asks which version of each tool and service the project uses, one question at a
time, the recommended one first: **Rust** (written to `rust-toolchain.toml`; installed toolchains
are listed first), the **Topcoat CLI** revision, and **Postgres**, **Redis**, **MongoDB** and
**Keycloak** (the image tag in `docker-compose.yml`), each with the latest and the common older
versions. Esc goes back to the wizard. The same choices are flags: `stackup new shop --pin
postgres=17 --pin rust=1.99.0` (`rust`, `topcoat`, `postgres`, `redis`, `mongodb`, `keycloak`).
Data made by one Postgres or MongoDB major version has to be migrated to open in another.

### Autofix

When something fails on the in-app terminal (an install, or any action in the menu) and the error is
one stackup recognises, an **Autofix** notice opens by itself: what went wrong, the exact commands
that would fix it, and that the failed steps are then run again. **Autofix** or **Close**; it never
runs without you choosing it, and the same fix is never offered twice in a row. It knows:

| Failure | Fix |
|---|---|
| Rust too old (`requires rustc 1.99 or newer`) | asks which Rust (installed ones, latest stable, common ones), gets it with `rustup`, then runs the step again with it |
| Docker is not running (macOS) | `open -a Docker`, wait until it answers, then the step again |
| `.env` is missing, and there is a `.env.local` | copies `.env.local` to `.env`, then the step again |
| Postgres exits with "in 18+, these Docker images…" (the volume is mounted at the old `/var/lib/postgresql/data`) | rewrites the mount in `docker-compose.yml` to `/var/lib/postgresql`, then the step again |

The Topcoat CLI install avoids the first one up front: it uses a new enough toolchain you already
have (`cargo +1.99.0 install ...`), and only updates Rust when there is none.

## Package versions

stackup installs the **latest** release of every package and crate, and the Docker images use
`latest`/`lts` tags. New projects start current, but a new major release can occasionally break
the generated code. One pin is deliberate: Nuxt projects use `typescript@6` until
`@typescript-eslint` supports TypeScript 7.

## How it's built

```
src/
  main.rs                 starts the command line
  commands/               the CLI: args (every command and flag), dispatch, project, create
  ui/                     the terminal app
    features/             one folder per part, with its pages, widgets, keys and state:
      home/                 the logo and the Start / My projects / Quit menu
      projects/             the list of projects, and one project's actions and addresses
      create/               the wizard, its preview, the tool checks, the install page
    components/ stores/   what every screen shares: modals, the palette, the app's state, and
                          the pane programs such as lazygit run in
    composables/          the app's logic, one file per concern: use_navigation, use_actions,
                          use_create, use_choices, use_envs, use_prompts, use_autofix, use_app_keys
    app_tests/            the menu's tests, one file per feature
  services/
    generate/             what `stackup new` writes: backend/, frontend/, compose/, tools/, workspace
    projects/             working with a project that exists:
      plan/                 what each action runs: containers, ship, environment, repo, project
      git/  ide/  envs  deploy  doctor  urls  certs  registry ...
    toolchain/            what this machine has (machine/), which versions can be installed
                          (versions), the GitHub setup (github), and the fixes offered (autofix)
  clients/                the only place that talks to the system: run (programs), pty, docker,
                          and os, everything that differs on macOS, Linux and Windows
  models/
    stack/                what is chosen: frontend, backend, service, tool, plan
    project/              what a project is and can do: facts, action, step
packaging/                Homebrew, Arch, Debian, Scoop and winget files for the releases
install.sh  install.ps1   the one-line installers for Linux/macOS and Windows
assets/                   files copied into generated projects, grouped by stack
```

A layer uses the ones below it and never the ones above (`ui` and `commands` → `services` →
`clients`), with `models` used by all and using none; `src/layers.rs` is a test that fails if an
import goes the wrong way. It's the same layering stackup generates for your projects. Every action becomes a list of
steps (`services/projects/plan/`); the command line runs them in your terminal and the menu runs
the same steps on its in-app terminal, so there is one place that knows what `up` or `deploy` means.

```sh
cargo test      # unit tests: wizard behavior, plan rules, generated files, the layering
cargo build --release
```

The tests run real programs where they can (git repositories, a terminal for the menu, `docker`
when it is there) and skip what a machine lacks. CI runs them on Linux, macOS and Windows.

## License

[MIT](../LICENSE). Free to use, modify and distribute, including commercially; just keep the copyright notice. Provided as is, without warranty.
