# Frontend Features

Reference for four browser-side features of the Sky Moon Scope web app. All code
lives under [`web/`](../web).

Shared conventions:

- Pure logic ships as plain `.js` modules with a hand-written `.d.ts`, so the
  unit tests (`node --test`, `.cjs`) exercise the real implementation instead of
  a copy. This mirrors the existing `lib/gasGolfingSort.js` pattern.
- Run everything with `npm test`, `npm run lint` and `npm run build` from `web/`.

---

## 1. Subgraph schema visualizer (issue #628)

An interactive node diagram of contract-to-contract calls and the ledger keys
each invocation touches, rendered with [React Flow](https://reactflow.dev).

| File | Role |
| --- | --- |
| `lib/schemaGraph.js` / `.d.ts` | Turns an `/analyze` report into `{ nodes, edges, stats }` |
| `components/SchemaVisualizer.tsx` | React Flow canvas, custom node types, minimap |
| `lib/schemaGraph.test.cjs` | Unit tests for the graph model |

**Where to find it:** the **Schema** tab on the home page, or `/?tab=schema`.

`buildSchemaGraph(report, options)` reads three parts of the report:

- `call_graph.root` — walked breadth-first into one node per unique
  `contract_id::function`, positioned by depth (column) and sibling index (row).
  Layout is deterministic, so the same report always draws the same diagram.
- `state_dependency` and `state_snapshot.ledger_entries` — rendered as storage
  nodes edged off the entry point. A key present in the post-simulation snapshot
  is labelled `writes`, otherwise `reads`.
- `ttl_analysis.touched_entries` — annotates storage nodes with remaining
  ledgers.

Re-entrant calls (A → B → A) are detected via the ancestor chain: the edge is
still drawn and labelled `re-entrant`, but the branch is not expanded again, so
cyclic graphs cannot hang the renderer. Storage nodes are capped
(`maxStorageNodes`, default 12) with the overflow reported in
`stats.hiddenStorageNodes`.

The component is mounted through `next/dynamic` with `ssr: false` because React
Flow measures real DOM nodes.

---

## 2. Global search shortcut (issue #623)

`Cmd+K` / `Ctrl+K` opens a quick-search overlay from anywhere in the app.

| File | Role |
| --- | --- |
| `lib/searchCommands.js` / `.d.ts` | Command registry, shortcut detection, ranking, parameter-hint builders |
| `lib/searchHistory.js` / `.d.ts` | Search history — persist/load recently visited commands from localStorage |
| `components/GlobalSearchModal.tsx` | The overlay; mounted once in `pages/_app.tsx` |
| `lib/searchCommands.test.cjs` | Unit tests for matching, keyboard navigation, and parameter hints |
| `lib/searchHistory.test.cjs` | Unit tests for history push/dedup/remove/clear |

The listener sits on `window` so the shortcut fires regardless of focus.
`Escape`, the backdrop and the close button all dismiss it; `↑`/`↓` move the
highlight (wrapping at both ends) and `Enter` runs the highlighted command.

Commands come from `buildCommandRegistry()`:

- **Navigation / Preferences** entries carry an `href` and are routed with
  `next/router`. Home-page panels are addressed as `/?tab=<id>`, which
  `pages/index.tsx` reads back out of the query string.
- **Contract function** entries carry an `action` instead. Selecting one
  dispatches a `Sky Moon Scope:search-command` `CustomEvent`, which the home page
  listens for — so the modal never needs to know page-internal state.

Ranking (`scoreCommand`), strongest first: exact title, title prefix, word start
within the title, keyword hit, title substring, subtitle substring, keyword
substring, then a loose subsequence fallback. Ties keep registry order so the
list does not reshuffle while typing.

Adding a command means appending one object to `BASE_COMMANDS`.

### Search history (recently visited panels)

When the query is empty the overlay shows the **Recently visited** section
instead of a blank list. Every time the user selects a command it is pushed to
the top of a LocalStorage-backed history list (key
`sky-moon-scope:search-history`, capped at 8 entries). The list deduplicates by
command `id` so revisiting a panel simply bubbles it to the top rather than
creating a duplicate entry.

```
lib/searchHistory.js   pushHistory / removeHistoryEntry / clearHistory
                       commandToEntry / historyToCommands
                       loadHistory / saveHistory (localStorage adapter)
```

`historyToCommands` converts stored entries back to `SearchCommand`-shaped
objects (with `group: 'Recent'`) so they flow through the existing
`filterCommands` ranking pipeline unchanged when the user starts typing.

### Contract function parameter type hints

Contract function commands in the registry now surface full parameter and
return-type information so developers can search by type or parameter name:

- **Title** — `transfer(from: address, to: address, amount: u128) → bool`
  (inline signature with optional `?` suffix for optional params).
- **Subtitle** — `Params: from: address — Sender address, to: address —
  Recipient address, amount: u128` (per-param description if present).
- **Keywords** — parameter names and Soroban types are added as search keywords,
  so typing `u128` or `address` in the search box surfaces every matching
  function.

Two helper functions are exported from `lib/searchCommands.js`:

| Function | Purpose |
| --- | --- |
| `buildParamHint(inputs)` | Formats `(name: type, …)` from a `ContractInput[]` |
| `buildFunctionSubtitle(fn)` | Builds a human-readable param listing for the subtitle |

---

## 3. User preference settings page (issue #622)

`/settings` lets power users point the app at self-hosted infrastructure.

| File | Role |
| --- | --- |
| `lib/userSettings.js` / `.d.ts` | Validation, normalization, LocalStorage I/O |
| `pages/settings.tsx` | The form, with per-endpoint connection tests |
| `lib/userSettings.test.cjs` | Unit tests |

Stored under the LocalStorage key `Sky Moon Scope-user-settings`:

| Field | Meaning |
| --- | --- |
| `rpcUrl` | Custom Soroban RPC endpoint; blank means the selected network's default |
| `indexerUrl` | Custom analyzer/indexer base URL; blank means `NEXT_PUBLIC_API_URL` |
| `requestTimeoutMs` | Timeout for the connection tests, clamped to 1000–120000 |

Nothing is sent to the backend — preferences never leave the browser.

Endpoints must be absolute `http://` or `https://` URLs; trailing slashes are
stripped on save. Reads are defensive: corrupt JSON, an unavailable store or a
stale schema all fall back to defaults rather than throwing.

**Test** buttons probe each endpoint — a JSON-RPC `getHealth` for RPC, `GET
/health` for the indexer. Any HTTP answer (including 404) counts as reachable,
since that still proves the host is live.

`lib/api.ts` resolves the base URL per request via `getApiBaseUrl()`, so a saved
`indexerUrl` takes effect immediately, without a rebuild. `components/upload-zone.tsx`
uses the same resolution for its analyze endpoint.

---

## 4. Web Worker for WASM decoding (issue #621)

Decoding uploaded bytecode on the main thread froze the UI. Parsing now runs in
a dedicated worker.

| File | Role |
| --- | --- |
| `public/wasm/wasmValidation.js` | The decoder — magic number, version, section walk, exports/imports, data segments |
| `public/wasm/wasmWorker.js` | Classic Web Worker wrapping the decoder |
| `lib/wasmValidation.js` / `.d.ts` | Re-export so app code and tests share one implementation |
| `hooks/useWasmValidationWorker.ts` | Lazily owns the worker, with a main-thread fallback |
| `lib/wasmValidation.test.cjs`, `lib/wasmWorker.test.cjs` | Decoder and worker-protocol tests |

### Why `public/`

The worker is served as a static file and pulls the decoder in with
`importScripts()` at runtime. That deliberately avoids bundler worker-entry
semantics, which differ between webpack and Turbopack — this way `next dev` and
`next build` behave identically. The UMD wrapper in `wasmValidation.js` lets the
same file be a worker global, a bundler import and a `require()` in tests.

### Protocol

```
→ { type: 'ping' }                                          ← { type: 'pong' }
→ { id, type: 'validate', buffer: ArrayBuffer, maxBytes? }   ← { id, type: 'result', report }
                                                             ← { id, type: 'error', message }
```

The hook boots the worker lazily (pages that never upload pay nothing) and only
sends real work after the `pong` handshake. If the worker cannot start — CSP, an
old browser, a 2s handshake timeout — validation silently runs the *same*
function on the main thread, so results are identical and an upload never fails
because of worker plumbing. Buffers are transferred rather than copied on the
happy path.

`validateWasmModule` never throws: decode failures come back as
`{ valid: false, errors: [...] }`, which is what crosses the message channel.
Beyond magic number and version it walks every top-level section and decodes the
export table, which is what `extractContractFunctions()` surfaces in the UI.

### Fix: worker crash on custom name sections (issue #409)

Soroban contracts compiled with debug info include a WASM custom section (id=0)
with the name string `"name"` and function-name subsection data. These are
larger, more realistic modules that triggered a latent boot-time crash in the
worker.

**Root cause.** `wasmWorker.js` originally contained this at the top level,
outside `onmessage`:

```js
const { validateWasmModule } = self.SkyMoonScopeWasmValidation;
```

`importScripts('/wasm/wasmValidation.js')` runs synchronously just before this
line and normally succeeds, populating `self.SkyMoonScopeWasmValidation`. But if
`importScripts` ever failed — 404, CSP header, network hiccup in the worker
scope — the destructure threw a `TypeError` at boot time, before `onmessage` was
registered. The ping handshake in `useWasmValidationWorker` therefore never
received a `pong`, the 2s timeout fired, and the hook silently fell back to the
main thread. Uploads still worked but the off-thread benefit was lost and any
future call with the dead worker would hang until fallback triggered again.

**Fix.** The destructure (now `const api = self.SkyMoonScopeWasmValidation`) was
moved inside `onmessage`, after the ping fast-path. The import is still attempted
at boot so the happy path is unchanged; only `validate` calls need the API
object. If it is absent they receive an `{ type: 'error', message: '...' }` reply
rather than the worker dying silently.

**Test fix.** `lib/wasmWorker.test.cjs` previously pre-populated
`sandbox.SoroscopeWasmValidation` (wrong global name) and used a no-op
`importScripts`. The sandbox now implements `importScripts` by running the
requested script file through `vm.runInContext`, exactly mirroring the real
worker boot sequence. Six regression tests were added covering the name-section
scenario, non-ASCII function names in the name section, and the importScripts-
failure graceful-error path.

---

## 6. Voice input for global search (issue #394)

The `Cmd+K` / `Ctrl+K` search overlay supports microphone input via the browser's
Web Speech API, letting users speak a query instead of typing it.

| File | Role |
| --- | --- |
| `lib/useVoiceSearch.ts` | Custom React hook; wraps `SpeechRecognition`, manages session lifecycle |
| `components/GlobalSearchModal.tsx` | Wires the hook; renders mic button, pulsing indicator, inline error hint |
| `lib/__tests__/useVoiceSearch.test.js` | State-machine unit tests (no React renderer needed) |

### How it works

`useVoiceSearch({ onTranscript })` returns `{ supported, listening, error, start, stop }`.

- **`supported`** — `false` in environments where `window.SpeechRecognition` and
  `window.webkitSpeechRecognition` are both absent (Firefox stable, SSR, Node.js).
  The mic button is hidden entirely in those cases, so the overlay is unaffected.
- **`listening`** — flips to `true` when a session starts and back to `false` on
  `onend` or `onerror`.
- **`error`** — set to the Web Speech API error code (`'not-allowed'`, `'no-speech'`,
  `'network'`, etc.) on failure. The `'aborted'` code is swallowed because it is
  fired when the hook calls `abort()` itself and does not represent a user-visible
  problem. Displays a one-word hint (`Mic blocked`, `No speech`, …) next to the
  close button.

### UI behaviour

- A **mic button** appears to the left of the close button.
- Clicking it starts a session; clicking again (or receiving `onend`) stops it.
- While listening, the static search icon is replaced with a **pulsing cyan mic**
  and the placeholder changes to `Listening…`.
- Recognised speech is written into the query field as interim results arrive, so
  results update in real time as the user speaks.
- Closing the modal (Escape, backdrop click, close button) calls `stop()`, which
  aborts any active session before unmounting.

### Graceful degradation

The hook accepts a `recognitionFactory` override (used by tests), and the default
factory checks `window.SpeechRecognition ?? window.webkitSpeechRecognition` at
call time — not at module load time — so it works correctly in SSR environments
where `window` is undefined.

### Testability

Because the hook state-machine is extracted from React (the tests drive it by
calling `startVoiceSearch` / `stopVoiceSearch` helpers with a mock recognition
object), the tests run under `node --test` with zero browser or renderer
dependencies. The mock fires synthetic `onresult`, `onerror`, and `onend` events
to cover every branch.

After a WASM file is decoded, a collapsible panel appears below each file card
in the upload list, giving a structured view of the module's binary contents.

| File | Role |
| --- | --- |
| `components/WasmSectionSummary.tsx` | Collapsible panel component with four tabs |
| `components/WasmSectionSummary.test.cjs` | Parser extension tests and panel logic tests |
| `public/wasm/wasmValidation.js` | Extended with `parseDataSection` and fixed `parseImportSection` |
| `lib/wasmValidation.d.ts` | New `WasmDataSegment` interface; `dataSegments` added to `WasmValidationReport` |

### Panel structure

`<WasmSectionSummaryPanel report={report} />` accepts a `WasmValidationReport`
and renders a collapsible panel (closed by default) with four tabs:

| Tab | Content |
| --- | --- |
| **Exports** | All exported symbols — name, kind (function / table / memory / global), index |
| **Imports** | All imported symbols — module, name, kind |
| **Data** | Data segment index, kind (active / passive), offset, byte length |
| **Sections** | All top-level section IDs and sizes |

Warnings from the decoder (e.g. "Module exports nothing") appear in an amber
footer strip at the bottom of the open panel.

The panel is integrated into `components/WasmUpload.tsx`: it mounts below the
file card for every file whose status is `"success"` and whose `validation`
report is available.

### Parser extensions

**`parseDataSection(bytes, start, end) → WasmDataSegment[]`** (new)

Parses the data section (id = 11). Each segment records its kind
(`"active"` / `"passive"` / `"unknown(n)"`), target memory index, offset
constant (decoded from `i32.const <imm> end`; `null` for passive or complex
offset expressions), and the byte length of the initialiser data. The raw bytes
are not stored to keep the JSON report small.

**`parseImportSection` (bug fix)**

Previously the import parser stopped at the first non-function import kind
(table / memory / global), because it only knew how to skip the one-byte type
index that follows a function descriptor. Each descriptor kind now has its bytes
correctly skipped (`skipImportDescriptor`), so all imports are decoded
regardless of kind mix.

### Types

```ts
interface WasmDataSegment {
  index: number;
  kind: 'active' | 'passive' | string;
  memoryIndex: number | null;
  offsetValue: number | null;
  byteLength: number;
}

interface WasmValidationReport {
  // ... existing fields ...
  dataSegments: WasmDataSegment[];
  sorobanMeta?: SorobanMetaDecodeResult;
}
```

---

## 5. Soroban metadata decoding from WASM custom sections (issue #413)

Parses Soroban-specific custom sections embedded in WASM binaries by the official
Soroban Rust SDK (`soroban-sdk`):

| Section | Content | Decoded representation |
| --- | --- | --- |
| `contractmetav0` | Build and SDK metadata (e.g. `rssdkver`, `rsver`) | Key-value pairs (`SorobanMetaKV[]`) |
| `contractspecv0` | Full contract ABI (functions, params, returns, types) | Functions (`SorobanSpecFunction[]`), types (`SorobanSpecType[]`) |

| File | Role |
| --- | --- |
| `public/wasm/sorobanMeta.js` | Zero-dependency UMD decoder for `contractmetav0` & `contractspecv0` |
| `lib/sorobanMeta.js` / `.d.ts` | App-facing entry point and TypeScript declarations |
| `components/SorobanMetadataPanel.tsx` | Interactive UI panel displaying ABI signatures, types, and build metadata |
| `lib/sorobanMeta.test.cjs` | Unit tests for XDR primitives, custom section search, and decoding |
| `components/SorobanMetadataPanel.test.cjs` | Tests for metadata filtering and signature generation |

### Integration

- **Off-thread validation**: `public/wasm/wasmWorker.js` loads `sorobanMeta.js` and decodes metadata inside the worker.
- **Section summary panel**: `components/WasmSectionSummary.tsx` displays a **Soroban Spec** tab whenever Soroban custom sections are detected in the uploaded WASM.
- **Backend decoder**: `core/src/soroban_meta_decoder.rs` provides the Rust counterpart using `soroban-sdk`'s native XDR reader.

