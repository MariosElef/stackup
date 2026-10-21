<div align="center">

# stackup

**Scaffold a full-stack project in one command, from the official tools.**

Nuxt or Vue · axum or FastAPI · Postgres, Redis, Mongo, Keycloak · Docker Compose · Makefile · CI

</div>

---

`stackup` is a small Rust CLI with a full-screen wizard. You pick a frontend, a backend, the
services you need and some dev tooling. It then runs the *official* scaffolders (`cargo new`,
`uv init`, `nuxi init`, Vite+'s `vp create vue`), adds the packages for what you chose, and writes
the glue around them: Docker Compose, a Makefile, a CI script, lint and format config, env files,
and a git repo.

It doesn't ship its own copy of a framework template. Every project starts from whatever the
official tool generates today, so what you get is current.

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
│ [x] Redis                      ││ ├─ Makefile                           │
│ ▶ Create project               ││ └─ ci.sh                              │
└────────────────────────────────┘└───────────────────────────────────────┘
 NORMAL  menu   needs: pnpm ✓  cargo ✓  git ✓
```

## Install

```sh
cargo install --git https://github.com/MariosElef/stackup
```

or from a clone:

```sh
git clone https://github.com/MariosElef/stackup && cd stackup
cargo install --path .
```

## Use

```sh
stackup new my-app          # opens the wizard
stackup new .               # scaffold into the current (empty) folder

# no wizard: give every choice as a flag
stackup new my-app --frontend nuxt --backend axum --libs pinia,colada,i18n \
  --services postgres,redis --tools makefile,gitleaks,lint,ci,git
```

Flags you pass pre-fill the wizard. Give `--frontend`, `--backend`, `--services` and `--tools`, and
it is skipped. Then:

```sh
cd my-app
make help     # every shortcut, with a one-line description
make up       # docker compose up --build
```

### Requirements

On your `PATH`, depending on what you pick: `pnpm` (all frontend work), `cargo` (axum), `uv`
(FastAPI), `git`, `docker`, and `mkcert` (local HTTPS). The wizard's footer shows what is installed
and won't create until the required tools are present. `gh` is optional and only used for
`--create-remote`.

## What you can pick

| Choice | Options |
|---|---|
| Frontend | `nuxt`, `vue`, `none` |
| Backend | `axum`, `fastapi`, `none` |
| Libraries | `pinia`, `colada`, `i18n`, `zod`, `openapi`, `lucide`, `tailwind`, `vueuse` (`--libs`) |
| Services | `postgres`, `redis`, `mongodb`, `keycloak`, `mailtrap`, `https` |
| Tools | `makefile`, `gitleaks`, `lint`, `ci`, `git` |

Pinia Colada is built on Pinia, so ticking it adds Pinia for you.

## What you get

Every project is layered the same way, so you always know where code goes. Empty folders hold a
`.gitkeep` (Python packages get an `__init__.py`).

```
backend (axum)     src/{routes,services,repositories,models,clients,middleware}/
backend (FastAPI)  app/{routes,services,repositories,models,schemas,clients,auth}/  + tests/
frontend (Nuxt)    app/{pages,components,services,stores,composables,…}/
frontend (Vue)     src/{views,components,services,stores,composables,…}/
```

The layering is **enforced**, not just suggested. A violation fails `ci.sh`:

- **axum:** routes and services can't touch `sqlx`, `redis` or `mongodb`; routes can't use `clients`.
- **FastAPI:** `import-linter` contracts: routes → services → repositories, and nothing below the
  routes imports `fastapi`.
- **Frontend:** oxlint `no-restricted-imports`: components can't import stores or services, and
  services can't import stores or UI.

### Libraries bring their own folders and wiring

| Library | Packages | Adds |
|---|---|---|
| Pinia | `pinia` (+ `@pinia/nuxt`) | `stores/` |
| Pinia Colada | `@pinia/colada` (+ `@pinia/colada-nuxt`) | `queries/` |
| i18n | `@nuxtjs/i18n` (Nuxt), `vue-i18n` (Vue) | Nuxt: `i18n/locales/en.json` and module config. Vue: `src/i18n.ts`, `src/locales/en.json`, wired in `main.ts` |
| Zod | `zod` | `contracts/` |
| OpenAPI client | `@hey-api/openapi-ts` | `contracts/`, `openapi-ts.config.ts`, `pnpm generate:api` |
| Lucide, VueUse | `@lucide/vue`, `@vueuse/core` | |
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
  Run `./scripts/certs.sh` once (`make up` does it for you). It uses 8443 because Docker Desktop's
  Kubernetes commonly holds 80 and 443.

### Configuration

Settings live in per-tier `.env` files. `.env.example` is committed; `.env.local` is generated and
copied to `.env`, which compose reads. Add `.env.dev` or `.env.prod` and switch with
`make env TIER=dev`. Everything except `.env.example` is gitignored.

### Makefile

A Makefile is just a list of named shortcuts. `make help` prints them:

```
make up       Build and start everything in Docker
make down     Stop everything
make dev      Start the databases in Docker and run the backend with reload
make ci       Run the same checks CI runs (format, lint, tests)
make fmt      Format backend and frontend code
make env      Switch environment: make env TIER=dev
make hooks    Turn on the gitleaks pre-commit hook
```

### Git

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

## Versions

stackup installs the **latest** release of every package and crate, and the Docker images use
`latest`/`lts` tags. New projects start current, but a new major release can occasionally break
the generated code. One pin is deliberate: Nuxt projects use `typescript@6` until
`@typescript-eslint` supports TypeScript 7.

## How it's built

```
src/
  main.rs                 the CLI: flags and the order things run in
  ui/tui.rs               the wizard (presentation)
  services/               what gets generated, one folder per concern
    frontend/             nuxt.rs, vue.rs, shared.rs
    backend/              axum.rs, fastapi.rs
    compose/              docker-compose.yml and the Caddy proxy
    tools/                git, Makefile, ci.sh, env files, README
  clients/run.rs          the only place that runs external programs
  models/                 the choices, one file each: frontend, backend, service, tool, plan
assets/                   files copied into generated projects, grouped by stack
```

Each layer only calls the one below it (`ui` → `services` → `clients`), with `models` shared by
all. It's the same layering stackup generates for your projects.

```sh
cargo test      # unit tests: wizard behavior, plan rules, generated files
cargo build --release
```

## Contributing

Issues and pull requests are welcome. To add a framework or service, add its choice under
`src/models/`, its generator under `src/services/`, and any files it copies under `assets/`.
