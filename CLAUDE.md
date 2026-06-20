# Silvie — CLAUDE.md

## Project overview

Silvie is an AI-powered personal assistant for executives on the road. It acts as a unified command center for trips, meetings, bookings, receipts, and follow-ups — think "AI Chief of Staff for business travel and executive logistics." See `README.md` for the full product vision.

Current state: hello-world scaffold. The product vision is defined but the features are not yet built.

## Tech stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 |
| Frontend framework | SvelteKit 2 + Svelte 5 |
| Language | TypeScript (frontend), Rust (Tauri shell + standalone backend) |
| Package manager | pnpm |
| Bundler | Vite 6 |
| Backend HTTP | `poem` (Rust) |
| AI framework | `rig-core` (Rust) |
| LLM provider | Gemini (`gemini-2.0-flash`) over its free tier |
| App ↔ backend protocol | Server-Sent Events |

## Repo structure

```
silvie/
├── Cargo.toml            # Workspace root — members: src-tauri, server
├── src/                  # SvelteKit frontend
│   ├── routes/           # File-based routing (+page.svelte, +layout.ts)
│   ├── lib/
│   │   ├── components/   # Svelte components
│   │   ├── services/     # Frontend services (incl. chat.ts SSE client)
│   │   ├── stores/       # Svelte 5 runes stores
│   │   ├── data/         # Static data (connectors, preferences, events)
│   │   └── types.ts
│   └── app.html
├── src-tauri/            # Rust / Tauri desktop shell
│   ├── src/{main.rs, lib.rs}
│   ├── Cargo.toml
│   └── tauri.conf.json
├── server/               # Standalone Rust HTTP backend (poem + rig + Gemini)
│   ├── src/{main.rs, server.rs, chat.rs, llm.rs, types.rs}
│   ├── Cargo.toml
│   └── .env.example
├── static/
├── .env.example          # frontend env (VITE_SILVIE_SERVER_URL)
├── package.json
├── svelte.config.js
├── vite.config.js
└── tsconfig.json
```

## Dev commands

The app needs **two processes running**: the Rust backend (LLM proxy) and the Tauri shell.

```bash
# ── Terminal 1: backend (LLM proxy) ───────────────────────────────────────
cp server/.env.example server/.env   # first time only — then fill GEMINI_API_KEY
cd server && cargo run               # listens on http://127.0.0.1:8080
# or, containerised (also reads server/.env):
docker compose up --build            # from repo root

# ── Terminal 2: Tauri desktop app ────────────────────────────────────────
pnpm tauri dev                       # full app
# or:
pnpm dev                             # browser-only frontend at http://localhost:5173

# ── Other ─────────────────────────────────────────────────────────────────
pnpm check                           # type-check frontend
cargo check --workspace              # type-check all Rust crates
pnpm tauri build                     # build distributable desktop app
```

The chat UI talks to the backend at `http://localhost:8080` by default. Override with `VITE_SILVIE_SERVER_URL` in a root `.env`.

## Key config

- Tauri dev URL: `http://localhost:1420`  
- Frontend output dir (for Tauri): `../build` (via `@sveltejs/adapter-static`)
- App identifier: `com.silvie`
- Default window: 800×600

## Testing / previewing

The Claude Code built-in preview tool does not work with Tauri — always ask the user to test the app manually with `pnpm tauri dev` (full desktop app) or `pnpm dev` (browser-only frontend at http://localhost:5173). Do not attempt to use the preview tool or take automated screenshots.

## Verifying Rust changes

Before claiming any Rust change is complete, run `cargo check --workspace` (or `cargo check -p <crate>` for a single crate). Type-checking is fast and catches the API-mismatch errors that are easy to introduce when writing against unfamiliar crates. Do not rely solely on inspection — always have the compiler verify.

## Rust tracing conventions

Both crates have `tracing` + `tracing-subscriber` wired up. New Rust code must instrument itself from the start — runtime failures are otherwise invisible.

**Subscriber setup (already done — reference only)**
- `src-tauri`: initialised in `run()` via `tracing_subscriber::fmt().with_env_filter(...)`, default filter `silvie=debug`
- `server`: initialised in `main.rs`, same pattern
- Override at runtime: `RUST_LOG=silvie=debug,silvie_server=debug`

**Rules for new Rust code**
- Add `#[instrument]` to every non-trivial `async fn`; use `skip(param)` for secrets and large types
- Log entry of every Tauri command and HTTP handler at `info!`
- HTTP calls: log URL + status at `debug!`; on **error** log the full response body at `error!`; on success body is `debug!` only (may be large)
- Use `info!` for lifecycle milestones (server start, auth flow steps, keychain writes)
- Use `debug!` for intermediate values useful during development
- Use `warn!` for recoverable failures (timeouts, cancelled flows, retries)
- Use `error!` for hard failures, always with the full error: `error!("thing failed: {e}")`
- **Never log secret values** (tokens, API keys, passwords) — log lengths instead: `token_len = token.len()`

See `src-tauri/src/auth.rs` for a reference implementation following all these rules.

## Terraform

- NEVER apply the terraform code.

## Data encryption policy

- **No client-side / application-level encryption (no KMS envelope encryption).**
  Earlier drafts of the schema marked sensitive fields (passport numbers, OAuth
  tokens) as KMS-encrypted via a per-user DEK. That was dropped to reduce
  complexity. We rely on:
  - **Postgres encryption-at-rest** in production (RDS / Cloud SQL / Aurora —
    enabled by default on managed offerings).
  - **TLS in transit** between client ↔ server ↔ database.
  - **OS keychain** for any secrets the desktop client must hold (Auth0
    refresh tokens, Google OAuth tokens).
- Revisit this only if compliance requirements appear (e.g., a customer
  contractually requires application-level encryption) or we add a new
  category of secret materially more sensitive than what's already stored.

## Conventions

- SvelteKit static adapter — no server-side rendering; the build output is a static bundle consumed by Tauri.
- The `server/` crate is the LLM proxy. It exposes `POST /chat` (SSE) and `GET /health`. Provider/agent logic stays in `server/src/llm.rs`; the rest of the server is provider-agnostic.
- Rust **Tauri** commands exposed to the frontend go in `src-tauri/src/lib.rs` and are registered in the `tauri::Builder`. Don't put model/LLM logic here — that belongs in `server/`.
- The frontend talks to the backend through `src/lib/services/chat.ts` (streaming) — components never `fetch` directly.
- Keep concerns cleanly separated: UI logic in `src/`, OS/system integrations in `src-tauri/`, AI/network in `server/`.
