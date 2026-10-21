# Contributing to Tintenfisch

Thanks for helping. This guide covers the setup, how the code is organized and what a pull request needs before it can be merged.

## Setup

You need:

- Node.js 24 and pnpm (run `corepack enable` once, the version is pinned in `package.json`)
- Rust stable with the WebAssembly target: `rustup target add wasm32-unknown-unknown`

Then:

```sh
pnpm install
pnpm dev
```

`pnpm dev` builds the engine to WebAssembly first and then starts Vite. `pnpm tauri dev` opens the same app in the desktop shell.

## Project layout

```
engine/
  core/        the PDF engine in Rust, with all document logic and its tests
  wasm/        the thin WebAssembly binding, it only translates
desktop/       the Tauri shell that ships the web build as a macOS and Windows app
src/
  routes/      pages (TanStack Router file routes)
  components/  React components, grouped by area, shared building blocks in components/ui
  hooks/       React hooks without their own state
  stores/      Zustand stores, one per area
  services/    everything that talks outward: engine client, IndexedDB, network, browser APIs
  worker/      code that runs inside the Web Worker next to the engine
  lib/         logic without React and without I/O
  types/       protocol.ts, the messages between the UI and the engine
  styles/      app.css with Tailwind and the design tokens
tests/         frontend tests, mirroring the paths in src
```

Dependencies point one way: `routes > components > hooks > stores > services > lib > types`. The worker only shares `types` with the UI.

## Code rules

- The engine owns the document. The UI holds no document model and talks to the engine only through `src/types/protocol.ts`.
- Import through the `@/` alias, never with relative paths, and never with dynamic `import()`. Workers and assets come in through Vite's `?worker` and `?url`.
- One component or class per file, named after it. Components and classes use PascalCase file names, everything else camelCase.
- Style with Tailwind utilities and the tokens from `src/styles/app.css`. Reuse the building blocks in `src/components/ui` instead of restyling buttons, fields or dialogs.
- Never select elements by CSS class in logic. Use `data-` attributes for that.
- Keep code self-explanatory: no code comments and no Rustdoc. Pick names that say what the code does.
- Write identifiers, commit messages and code-related docs in English.
- Keep it lean: no abstractions for later, no interfaces or traits with only one implementation, no generic names such as `utils` or `helpers`. A file approaching 400 lines is the cue to split it.

## Tests

- Engine: `cargo test` in `engine/`, with fixtures under `engine/core/tests`.
- Frontend: `pnpm test` runs Vitest on `tests/`. Store tests connect the engine client to a fake port with `FakeEnginePort.attach()`, and every store is reset after each test.

Add or update tests with every behavior change. Do not use private or customer documents as fixtures; build small synthetic PDFs instead.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/) with an optional scope, for example `feat(viewer): ...`, `fix(edit): ...`, `refactor: ...`, `test: ...`, `docs: ...`. Keep unrelated changes in separate commits.

## Pull requests

Before you open one, run everything CI runs:

```sh
pnpm lint
pnpm test
pnpm build
cd engine && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd ../desktop && cargo fmt --check && cargo clippy --all-targets -- -D warnings
```

For changes to the interface, check them in the browser on desktop and on a narrow mobile width, and add screenshots to the pull request.

## Reporting bugs and ideas

Use the issue forms on GitHub. Please never attach confidential PDFs to an issue; describe the problem or share a file that contains nothing private. Security problems go through the process in [SECURITY.md](SECURITY.md), not through public issues.
