# AGENTS.md — Engineering Contract for QuicKlick

Binding on every agent and human who adds, modifies, or refactors code here. It is a contract,
not a guideline. If the code and this document disagree, the code is wrong.

**Read this file completely before your first edit.** Reading it after the edit violates R5.1.

---

## 1. Purpose & Scope

QuicKlick is a cross-platform desktop autoclicker: a Tauri v2 shell wrapping a Rust backend
(`src-tauri/`, edition 2021) and a SvelteKit 2 / Svelte 5 runes frontend (`src/`, TypeScript
`strict`, SPA via `adapter-static`).

In scope: `src-tauri/`, `src/`, and the build config `package.json`, `vite.config.js`,
`svelte.config.js`, `.prettierrc.yaml`. Out of scope and never hand-edited: `node_modules/`,
`build/`, `src-tauri/target/`, `src-tauri/gen/`.

**R1.1** If a task cannot be done without violating a rule here, stop and ask. Never silently
reinterpret a rule, and never grant yourself a one-time exception.

Where existing code violates a rule it is marked **[DEBT]**. You must not copy the pattern, must
not deepen it, and must not clean it up unless the current task touches that code. Mention the debt to the user.

### Repository Map

- **Backend** (`src-tauri/src/`) — `lib.rs`: `AppState`, action routing, wiring. `automator.rs`:
  input synthesis (`ClickTarget`, `Device`, `MouseButton`, `ClickType`). `scheduler.rs`: CPS
  delay, jitter, interruptible waits. `click_loop.rs`: the click thread and its run loop.
  `shortcuts.rs`: hotkeys via `wayland` / `evdev` / `standard`. `settings.rs`: `settings.json`,
  `AppSettings`, `SavedState`, presets. `frontend_api.rs`: Tauri commands, wire payloads,
  `AppStateDto`. `utils.rs`: `KeyCode`, `Modifier`, `InputEvent`, `MacroTimer`, platform probes.
- **Frontend** (`src/`) — `lib/clickerState.svelte.ts`: the single state holder and IPC client.
  `lib/HotkeyInput.svelte`: key-capture input. `constants.ts`: option lists.
  `routes/{,advanced,sequence,settings}`: one screen per route.

### Command Reference

Run from the repo root unless stated. The baseline is the state as of this document and is the bar
you must hold.

| Purpose             | Command                                                     | Baseline                                     |
| ------------------- | ----------------------------------------------------------- | -------------------------------------------- |
| Type / svelte check | `npm run check`                                             | 0 errors, 0 warnings — must stay 0/0         |
| Build + format      | `npm run build`, `npx prettier --check src`                 | succeed; `.prettierrc.yaml` is authoritative |
| Rust compile        | `cd src-tauri && cargo check --all-targets`                 | clean                                        |
| Rust format         | `cd src-tauri && cargo fmt --check`                         | clean                                        |
| Rust lint           | `cd src-tauri && cargo clippy --all-targets -- -D warnings` | **must remain 0**         |
| Rust tests          | `cd src-tauri && cargo test`                                | none yet; new logic must add one             |

---

## 2. Core Principles

### 2.1 Robustness

- **R1.2** No silent failure. Every `Result`, `Option`, and syscall outcome is handled
  explicitly, surfaced to the user, or propagated with added context. Discarding an error is
  prohibited. `let _ = fallible();` is allowed only inside a documented best-effort block whose
  comment states the consequence of ignoring it; this is also allowed for errors that shouldn't affect the user such as a single dropped click, such usage must be mentioned to the user.
- **R1.3** `unwrap()` and `expect()` are prohibited on any release-reachable path. Release sets
  `panic = "abort"` (`src-tauri/Cargo.toml`), so a panic kills the app with no recovery. Use
  `?`. If an invariant is truly infallible, prove it in a comment and supply a defined fallback
  instead of panicking.
- **R1.4** Validate every external input at the boundary, before it reaches state or the
  scheduler: user input, Tauri IPC payloads, `settings.json`, env vars, filesystem paths. Ranges
  are named constants shared between frontend and backend, never duplicated literals.
- **R1.5** No dead code, no commented-out blocks, no `TODO`/`FIXME`/`XXX`/`HACK` in merged code.
  An unfinished idea is an issue or a branch.
- **R1.6** Never claim what you did not verify: report the command you ran and its output.
- **R1.7** Code must work on the first run in the target environment: Linux/Wayland and
  Linux/X11 are supported, and every other platform must degrade rather than panic.

### 2.2 Future-Proofing

- **R2.1** A change must not harm, block, or complicate what comes after it. Before writing,
  name the next likely change to this area and check your design does not impede it. State that
  check in your summary.
- **R2.2** Prefer additive change. New behavior extends existing behavior. Rewriting working
  code to make room for a new requirement is prohibited unless that rewrite is the task.
- **R2.3** No premature abstraction — no generic frameworks, trait hierarchies, or plugin
  systems for a single concrete use. **[DEBT]** `ClickerState`
  (`src/lib/clickerState.svelte.ts`, 468 lines) has 15 near-identical getter/setter paifghfghfghfghfghfgh
  add a 16th without also considering extraction, and do not rewrite it unasked.
- **R2.4** No hard-coded shortcuts that must be torn out later. Anything varying by
  environment, platform, or configuration is a setting, a constant, or a `#[cfg]`-gated
  implementation — never a literal buried in a function. **[DEBT]** `vite.config.js` `fs.allow`
  contains `"../../newcomplib/Bluenite"`, a machine-specific path. Never add one.
- **R2.5** These interfaces are contracts and may not change casually: Tauri command and argument
  names (`set_cps_cmd`, `set_target_cmd`, …); event names (`state_change`, `error`,
  `shortcut_triggered`); serialized field names in `AppStateDto`, `AppSettings`, `SavedState`,
  `AppState`; the on-disk `settings.json` schema; the `@hermitk/bluenite` component props this
  app consumes; the `ClickTarget`/`SeqTarget` shape in `automator.rs`.
- **R2.6** A change to anything in R2.5 must be backward-compatible, or explicitly versioned
  _and_ migrated. A new field on a persisted struct requires `#[serde(default)]` so existing
  `settings.json` files keep loading. A rename or removal requires a migration in
  `settings::SettingsManager::new`.
- **R2.7** A rename is not a refactor. Do it in one commit across all call sites, and keep the
  wire name unchanged unless the task says otherwise.

### 2.3 Scalability & Modularity

- **R3.1** Separate concerns into modules with minimal interfaces. A module's public surface is
  what other modules need and nothing more. If `pub` can be `pub(crate)`, make it `pub(crate)`.
- **R3.2** Every module must be independently testable. A unit that cannot be tested without a
  live window, a real mouse, or a running Tauri app is doing too much; push the logic out until
  it is not.
- **R3.3** No hidden global state. Dependencies are passed in explicitly. Tauri managed state is
  the one sanctioned exception: `AppState` and `Arc<SettingsManager>` are reached through
  `app.state::<T>()` / `resolve_state`, never cached in a module-level static.
- **R3.4** Prefer composition over inheritance. In Rust, hold a collaborator as a field or pass it
  as a parameter; never build a trait hierarchy to share a few lines of behavior.
- **R3.5** The design must survive more data (longer sequences, many presets), more users
  (multiple displays, keyboard-only, non-Linux), and more contributors (someone who has never
  read this file). A design that works only at one machine's scale, one display's geometry, or
  one author's mental model is rejected.
- **R3.6** No unbounded growth per iteration in any buffer, history, or persisted collection;
  state the bound in code. **[DEBT]** `state.sequence` and `settings.presets` are unbounded today;
  adding a third growth point is prohibited.

### 2.4 Simplicity First

- **R4.1** Choose the simplest solution that fully solves the problem _and_ satisfies R1–R3.
  Simplest never means cutting a corner on correctness.
- **R4.2** Reject speculative generality. Do not build for requirements nobody stated. Config
  flags, pervasive `Option`, and strategy registries for a single known use case are prohibited.
- **R4.3** Every line must justify its existence: if you cannot name the reason in one clause,
  delete the line. Add no defensive layer for a case that cannot occur.
- **R4.4** Prefer the standard library and existing project utilities over new dependencies. A new
  dependency requires written justification in the change description: what it replaces, why std
  or an existing dependency cannot do it, and its maintenance and license status. A convenience
  import is not a justification, and a dependency must never ride along in a feature change.
- **R4.5** Reuse before building. UI primitives come from `@hermitk/bluenite`, icons from
  `lucide-svelte`, key and modifier parsing from `utils.rs`. Never hand-roll a component, icon, or
  parser that already exists in the project.
- **R4.6** Deleting code is a valid and preferred outcome. Delete a workaround rather than
  layering a new one on top of it.

### 2.5 Agent Behavior

- **R5.1** Read this document completely before your first edit, and re-read the relevant section
  before each subsequent edit to the same area.
- **R5.2** Never invent requirements. If the task is ambiguous, underspecified, or conflicts with
  this document, stop and ask. Do not pick an interpretation and proceed.
- **R5.3** Run the checks in the Command Reference before declaring the task complete, and report
  honestly: quote a failing check's output and say so when a check could not be run. Reporting
  completion without running them is a false claim, prohibited by R1.6.
- **R5.4** Modify only files within scope. Drive-by refactors, reformatting of untouched files, and
  "while I was in there" cleanups are prohibited even when correct. Leave unrelated pre-existing
  findings for a dedicated change.
- **R5.5** Prefer editing an existing file over creating a new one. A new file requires a stated
  reason it cannot live in an existing module.
- **R5.6** Leave the codebase so the next agent can continue without your context. Every
  non-obvious decision, deliberate deviation, and known limitation must be visible in a code
  comment, the commit message, or the task summary. Silent design decisions are prohibited.
- **R5.7** Never commit secrets, machine-specific paths, or editor state. Never hand-modify
  `src-tauri/gen/`, `build/`, `node_modules/`, or `package-lock.json`.

---

## 3. Code Standards

### 3.1 Formatting & Naming

- **S1.1** Frontend formatting is owned by `.prettierrc.yaml`; Rust formatting is owned by default
  `rustfmt`, and you must never add a `rustfmt.toml` to change those defaults. Run
  `npx prettier --write` and `cargo fmt` on every file you touch.
- **S1.3** Rust: `snake_case` functions and modules, `PascalCase` types and traits,
  `SCREAMING_SNAKE_CASE` constants. Every `#[tauri::command]` fn ends in `_cmd` and is registered
  in the `generate_handler!` list in `lib.rs` in the same change.
- **S1.4** TypeScript: `camelCase` values and functions, `PascalCase` exported components and
  types, `.svelte.ts` suffix for rune-based reactive modules.
- **S1.5** Name the domain concept, not the implementation: `set_cps_cmd` is right,
  `handle_click_2` and `doThing` are not.

### 3.2 Structure

- **S2.1** One responsibility per function. A function needing a comment to explain its "and"
  clause is two functions.
- **S2.2** `+layout.svelte` is a layout. Screen logic belongs in the route's `+page.svelte`; logic
  shared by two or more screens belongs in `src/lib/`.
- **S2.3** A `+page.svelte` must not construct IPC payloads inline; the wire shape belongs in the
  state layer so it has one definition in the frontend. **[DEBT]** both
  `src/routes/sequence/+page.svelte` and `src/lib/clickerState.svelte.ts` build
  `SeqTargetPayload`; never add a third copy.
- **S2.4** `match` arms stay exhaustive. A new enum variant must not compile until every `match`
  handles it. Never add `_ =>` to silence that.

### 3.3 Error Handling

- **S3.1** Return `Result<T, E>` and propagate with `?`. Convert external errors (`enigo`,
  `tauri`, `serde`) into a message naming the operation — `"failed to initialize input
automation: {e}"`, never a bare `e.to_string()`.
- **S3.2** A user-actionable failure must reach the user, in the backend via
  `app.emit("error", message)`. **[DEBT]** `AppState::emit` (`lib.rs:77`) discards the emit
  result, making a failed state broadcast invisible; report any place this hides a real failure.
- **S3.3** In the frontend, `console.error` alone is prohibited — this is a desktop app and the
  user cannot see the console. Use `toast.show({ variant: 'danger' })` from
  `@hermitk/bluenite`. **[DEBT]** `ClickerState.invoke` and `ClickerState.init` log to console
  only; when you touch them, add a toast.
- **S3.4** A frontend `catch` must do one of three things: surface a toast, restore local state
  to match the backend, or re-throw with context. A `catch` that only logs and continues
  degraded must justify why continuing is correct.
- **S3.5** Validation failures return a typed error (`Errors` in `lib.rs`, `Result<_, String>`),
  never a silent no-op or a substituted default.

### 3.4 Logging

- **S4.1** Use `eprintln!` for diagnostics. `println!` and `dbg!` are prohibited in app and
  library code — this is a GUI app with no attached stdout. **[DEBT]** the `println!` at
  `frontend_api.rs:71` fires on every target set; remove it when you touch that function.
- **S4.2** Log at most once per failure, at the point of detection, naming the operation and the
  cause; never re-log the same error in every layer it passes through. Never log the click loop's
  steady state — it runs hundreds of times per second. The `EMIT_INTERVAL` throttle in
  `click_loop.rs` is the model.

### 3.5 Testing

- **S5.1** New logic in `scheduler.rs`, `utils.rs`, `automator.rs` (parsing and validation),
  `settings.rs` (serialization, defaults, migration), or `shortcuts.rs` (matching) must ship
  with a `#[cfg(test)] mod tests` in the same file. These are pure and cheap to test.
- **S5.2** Tests assert behavior, not implementation: never a private field, a log line, or a call
  order the design does not otherwise promise.
- **S5.3** No test may depend on a real mouse, keyboard, display, network, wall-clock timing, or a
  running Tauri app. Inject a clock or a value; never sleep. **[DEBT]** the unexplained
  `sleep(Duration::from_secs(1))` at `click_loop.rs:16` is a bug to fix, not a pattern to copy.
- **S5.4** The frontend has no test runner. Never add one inside an unrelated change. If a task
  needs frontend tests, stop and ask and propose adding Vitest as its own change.
- **S5.5** Every bug fix includes a regression test that fails before the fix, unless S5.3 makes
  one impossible — in which case you must say so explicitly.

### 3.6 Concurrency

- **S6.1** The click loop runs on its own thread (`click_loop::run`). Never block the Tauri command
  thread or the webview with a sleep, a held lock across `.await`, or a synchronous input call.
- **S6.2** Global lock order: **`AppState` before `SettingsManager`.** **[DEBT]**
  `frontend_api::update_saved_state` inverts this by holding the settings lock inside
  `sm.update(..)` and taking `state.target.lock()` in the closure. Never extend that pattern,
  never reverse the order, and prefer `try_lock` inside any loop.
- **S6.3** Every atomic uses `Ordering::SeqCst` today. Match it. Do not weaken an ordering to
  "optimize" without a measurement and a comment citing it.
- **S6.4** `f64` is stored as bits in an `AtomicU64`. Never add new `f64` state that way: it admits
  NaN unchecked, carries no type, and needs manual conversion at every read. Use a `Mutex<f64>`
  or a bounded integer. **[DEBT]** `AppState::cps`.
- **S6.5** The click loop must stay responsive to a `Stop` signal. Any new wait in
  `click_loop.rs` goes through `Scheduler::wait_or_stop` or is proven non-blocking.

---

## 4. Architecture Rules

### 4.1 Module Boundaries & Dependency Direction

**A1.1** Dependencies point inward only, per this graph, and no other edge is permitted. Frontend:
`routes/*` → `$lib/clickerState` → `@tauri-apps/api`, never reversed. Backend: `frontend_api`,
`click_loop`, and `lib.rs` → `automator` / `scheduler` / `utils`; `shortcuts` → `utils`;
`settings` → `shortcuts` and `automator`.

**A1.2** `automator.rs`, `scheduler.rs`, and `utils.rs` are leaves. They must not import from
`lib.rs`, `frontend_api.rs`, `settings.rs`, or `shortcuts.rs`, and must not hold Tauri state.
**[DEBT]** `automator.rs:3` imports `frontend_api::ClickTargetPayload`, inverting the edge; new
payload types belong in the leaf that owns the domain, with `frontend_api.rs` converting. Never
extend that inversion.
**A1.3** Never reach into another module's fields to mutate state; mutate through a method on the
owning type.
**A1.4** `lib.rs` owns wiring only: `AppState`, action routing, plugin registration, command
registration. Business logic in `lib.rs` is misplaced.

### 4.2 Frontend State Ownership

- **A2.1** `src/lib/clickerState.svelte.ts` is the **single** source of truth for clicker state.
  Routes read and write it and must not hold a duplicate of a backend-owned value.
- **A2.2** All IPC goes through that module; a route must never `invoke` for clicker state. The
  read-only capability probes `is_wayland_cmd` / `is_linux_cmd` are the one exception, as they
  already are in `src/routes/settings/+page.svelte`.
- **A2.3** The backend is authoritative; the frontend mirrors it and never computes it
  independently. `state_change` is the push channel and `get_app_state_cmd` the pull channel.
  Never add a third.
- **A2.4** Transfer enums as strings or as a discriminated union with `as const`. Never transfer
  `mode: u8` and branch on `0`/`1` in two languages. **[DEBT]** `utils::MODE_NORMAL` /
  `MODE_SEQUENCE` exist and are ignored by `click_loop.rs`, `frontend_api.rs`, and the frontend;
  replace the literals with the constants in any change that touches mode.
- **A2.5** Never export a new module-level mutable singleton. Extend `clickerState` or add a pure
  function module.

### 4.3 Platform Support

- **A3.1** Platform code lives in `#[cfg(target_os = ...)]` blocks or cfg-gated submodules
  (`shortcuts::wayland`, `::evdev`, `::standard`). Environment probing lives in `utils.rs`
  (`is_wayland`, `is_linux`) and nowhere else; never re-read `XDG_SESSION_TYPE` or
  `WAYLAND_DISPLAY` in a second place.
- **A3.2** Every code path must compile on every target in `Cargo.toml`, including the untested
  non-Linux ones. Guard with `#[cfg]`; never `unreachable!()` around a non-Linux build.
- **A3.3** The `target.'cfg(target_os = "linux")'` block in `Cargo.toml` is the only place
  Linux-only crates may be declared, and Wayland and X11 must both keep working — any change to
  shortcut registration must state in the summary which backend it affects.

### 4.4 Interface Stability

- **A4.1** Wire-format changes are additive by default. A new `AppStateDto` field is `Option<T>`
  or carries `#[serde(default)]`, and the frontend type treats it as optional until the backend
  ships — the frontend must tolerate an older backend.
- **A4.2** The version is declared in **three** files: `package.json`, `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json`. A release bumps all three in one change. If there is a divergence, ask the user and work based on the provided response.
- **A4.3** `settings.json` is user data. New field: `#[serde(default)]`. Removed field: ignore its
  data, never delete it. Renamed field: a deserializer alias. No migration may discard a user's
  presets or shortcuts.
- **A4.4** Loading a corrupt or unreadable `settings.json` falls back to defaults, tells the user,
  does not panic, and does not overwrite the file until a real change occurs. **[DEBT]**.
  `SettingsManager::new` falls back silently and `save()` overwrites.

---

## 5. Definition of Done

Complete only when every box is checked. An unchecked box means not done.

**Scope**

- [ ] Only files required by the task were modified — no drive-by edits, no reformatting of
      untouched files (R5.4), no new file without a stated reason (R5.5) — and no dead code,
      commented-out block, or `TODO`/`FIXME` added, with existing ones in touched lines removed
      (R1.5).

**Correctness**

- [ ] New or changed logic in a leaf Rust module has `#[cfg(test)] mod tests` covering it (S5.1),
      asserting behavior not internals (S5.2), with no real input device, network, or wall-clock
      sleep (S5.3). A bug fix includes a failing-before regression test, or an explicit statement
      of why S5.3 makes one impossible (S5.5).
- [ ] Every external input crossing a boundary is validated with named range constants (R1.4).
      User-visible failures reach a toast, not only the console (S3.2, S3.3). No new `unwrap()`
      on a release-reachable path (R1.3).

**Verification**

- [ ] Every command in the Command Reference was run in this working tree and its output reported.
      Clippy must introduce **no new finding**; compare by name against the 13-finding baseline,
      never by count, since the count moves with the toolchain. Do not fix them in this change.

**Architecture**

- [ ] New dependencies point inward only; no leaf module gained one (A1.2). Lock order not
      inverted and no lock held across `.await` (S6.2).
- [ ] New state went into the owning module, not a new global (R3.3, A2.5). Every new command is
      registered in `generate_handler!` (S1.3). Persisted structs gained `#[serde(default)]` for
      new fields (A4.3, R2.6). Non-Linux builds still compile (A3.2).
- [ ] No dependency added without the R4.4 justification in the summary. No magic number where a
      constant exists; mode uses `utils::MODE_*` (A2.4). No new `f64`-as-bits atomic (S6.4). No
      unbounded growth (R3.6).

**Handoff**

- [ ] The summary states what changed, why, what was run and observed, what was deliberately left
      out, and the next likely change to this area (R5.6, R2.1). Every deviation from this document
      is stated explicitly, or the work was stopped and a question asked (R1.1, R5.2).

---

## 6. Prohibited Practices

Never do the following. Each is a review rejection.

**Correctness**

1. Swallow an error: `let _ = ...`, `.ok()`, a discarded `Err`, or an empty TS `catch {}`.
2. `unwrap()`, `expect()`, or any panic on a release-reachable path (`panic = "abort"`).
3. Log a user-affecting failure to `console`/`eprintln` and call it handled.
4. Write `println!` or `dbg!` in app or library code.
5. Trust frontend validation, IPC payloads, or `settings.json` without re-validating backend-side.
6. Ship `TODO`, `FIXME`, `XXX`, `HACK`, commented-out code, or dead code.
7. Add a `sleep` to mask a race, or copy the unexplained `click_loop.rs:16` sleep.
8. Detect the platform anywhere except `utils.rs`.
9. Put an absolute or machine-relative path in build config.

**Architecture**

10. Let a leaf module (`automator`, `scheduler`, `utils`) depend on an inner module.
11. Add a module-level mutable singleton, or a second source of truth for backend-owned state.
12. Call `invoke` from a route for clicker state.
13. Add a `mode: u8` literal instead of `utils::MODE_*` or a typed string.
14. Store `f64` as bits in an `AtomicU64`.
15. Hold two locks, hold a lock across `.await`, or invert the lock order.
16. Add unbounded growth to any loop, buffer, or persisted collection.
17. Add a field to a persisted or wire struct without a default and a migration.
18. Bump the version in only one of `package.json`, `Cargo.toml`, `tauri.conf.json`.
19. Introduce a trait, plugin registry, or config flag for a single known use case.

**Simplicity & process**

20. Add a dependency without the R4.4 written justification, or as a drive-by.
21. Hand-roll a UI primitive, icon, or key parser that bluenite, lucide, or `utils.rs` provides.
22. Refactor working code the task did not ask you to touch.
23. Declare a task done without running the §5 checks, or report an unrun check as passing.
24. Guess at a requirement instead of asking.
25. Commit `node_modules/`, `build/`, `target/`, `gen/`, secrets, or a hand-edited lock file.
