# Blog outline: Bringing Linux inference to Tiles

Use this as a writing guide for the **first** post in a series. This article explains *what we built and why the architecture looks the way it does*. A follow-up post can go deep on Harmony parsing, stop tokens, CUDA preload, and other llama.cpp internals.

**Working title ideas:**
- "Tiles on Linux: same agent, different inference engine"
- "How we added llama.cpp to Tiles without rewriting the product"
- "From MLX to llama.cpp: extending Tiles beyond macOS"

**Audience:** developers curious about local AI tooling, agent harnesses, or cross-platform inference design—not people who already know llama.cpp internals.

**Tone:** story + architecture. Show the problem, the constraint ("don't fork the product per OS"), and the seams where you plugged Linux in.

---

## 1. Hook — the problem (short)

Cover briefly:

- Tiles originally shipped inference on **macOS via MLX** (Apple Silicon–native, fast, good DX on Mac).
- Users on Linux had no path to run the same product locally.
- The goal was **not** "port Tiles to Linux" in the sense of rewriting everything—it was **bring Linux inference online while keeping one user experience**: same CLI, same Pi agent, same OpenResponses API Pi talks to.

**One sentence thesis for the reader:**
> We kept the orchestration layer stable and swapped only the inference backend per platform.

**Diagram you should draw:** before/after—macOS stack vs Linux stack, with the shared middle (Pi + OpenResponses API) highlighted.

---

## 2. What Tiles actually is (context for newcomers)

Explain the moving parts at a level a reader can hold in their head. Don't dive into implementation yet.

| Piece | Role |
|-------|------|
| **tiles CLI** (Rust) | Boots services, downloads models, runs the REPL |
| **Tiles daemon** (`:1729`) | Config + model cache path lookup |
| **Python inference server** (`:6969`) | OpenResponses-compatible HTTP API |
| **Pi** (embedded agent, RPC mode) | Agent loop, tools, reasoning effort; calls the API |
| **SQLite / ATProto** | Sessions, sharing—orthogonal to inference |

**Key insight to state explicitly:** Pi never talks to llama.cpp or MLX directly. It only sees `http://127.0.0.1:6969/v1` as an OpenResponses provider. That separation is why Linux was feasible.

**Diagram:** the four-process diagram (CLI → daemon + Python server + Pi). Arrows: user → REPL → Pi → `/v1/responses` → inference server.

---

## 3. The constraint that shaped the design

Cover these design decisions:

1. **One API contract** — Pi is configured with `openai-responses` and a `models.json` pointing at the local server. Linux and macOS must behave the same from Pi's perspective.
2. **Runner parity** — `LlamaRunner` was written to mirror `MLXRunner`'s interface so `linux.py` and `mlx.py` can share streaming logic patterns and `commons.py`.
3. **Runtime backend selection** — `server/main.py` picks `linux` or `mlx` based on `sys.platform`. No user-facing "backend" flag.
4. **GGUF on Linux** — default modelfile path differs (`modelfiles/gpt-oss-gguf` vs `modelfiles/gpt-oss` on Mac). HuggingFace download pulls `.gguf` + config, not MLX weights.

**What to avoid in this post:** long discussion of `n_gpu_layers`, KV cache, or CUDA wheel URLs—tease them for post #2.

---

## 4. End-to-end flow (the main narrative)

Walk through **one user message** from typing in the REPL to streamed output. This is the spine of the article.

### 4.1 Startup (first run)

1. `tiles` parses Modelfile (`FROM`, `SYSTEM`).
2. Starts **Tiles daemon** and **Python server** as background children.
3. If model missing → `pull_model` from HuggingFace into cache.
4. `POST /start` loads the model into memory (`get_or_load_model` → `LlamaRunner.load_model`).
5. Writes Pi `models.json` with `base_url: http://127.0.0.1:6969/v1`.
6. Spawns Pi in RPC mode (`PI_OFFLINE=true`).

**Diagram:** startup sequence (numbered steps). Optional: show retry path when partial download fails.

### 4.2 A single prompt (the hot path)

1. User types in REPL → JSON `{"type": "prompt", "message": "..."}` to Pi stdin.
2. Pi builds an OpenResponses request (history, tools, reasoning effort) → `POST /v1/responses` with `stream: true`.
3. FastAPI → `linux.generate_response_chat_stream`.
4. For **gpt-oss** models: `build_harmony_conversation` → `generate_streaming_gpt`.
5. For **other models**: `generate_streaming` (chat template path).
6. Server emits **SSE** events (`reasoning`, `message`, `function_call` deltas).
7. Pi forwards `MessageUpdate` text deltas → REPL prints live.
8. On `agent_end`, session saved to SQLite.

**Diagram:** sequence diagram for one prompt (REPL → Pi → server → runner → back).

**Call out:** streaming is the real path; non-streaming `generate_batch_gpt` exists for API completeness but Pi uses streaming.

---

## 5. Platform split — what changed vs what didn't

Use a two-column "unchanged / changed" section.

### Unchanged (shared across macOS and Linux)

- Rust CLI, REPL, daemon, model download plumbing
- Pi integration and RPC protocol
- `server/api.py` routes (`/ping`, `/start`, `/v1/responses`)
- `server/backend/commons.py` (Harmony conversation builder, SSE helpers, reasoning effort)
- OpenResponses streaming event shape Pi expects

### Changed per platform

| Concern | macOS | Linux |
|---------|-------|-------|
| Backend module | `server/backend/mlx.py` | `server/backend/linux.py` |
| Runner | `MLXRunner` | `LlamaRunner` |
| Model format | MLX weights | GGUF via llama-cpp-python |
| Default modelfile | `gpt-oss` | `gpt-oss-gguf` |
| Python deps | `requirements-macos.txt` | `requirements-linux.txt` (CUDA wheel) |
| Config tuning | MLX-specific | `[llama]` in `config.toml` (context, gpu layers, batch size) |

**Diagram:** Venn diagram or layered stack—shared API layer on top, platform runners below.

---

## 6. gpt-oss / Harmony — mention, don't deep-dive

gpt-oss is the default model family and it **does not** use a normal chat template. Briefly explain:

- OpenResponses input → `build_harmony_conversation` → Harmony `Conversation`
- Streaming uses `generate_streaming_gpt` + `StreamableParser`
- Output is normalized to markers Pi/`linux.py` already understand: `**[Reasoning]**`, `**[ToolCall]**`, `**[Answer]**`
- This normalization lets Linux match MLX behavior without Pi knowing which backend runs

**Defer to post #2:**
- `StreamableParser` channel semantics
- `ToolCallStart` vs string markers
- Harmony token rendering vs `create_chat_completion`
- Stop-token policy and EOS handling

**Diagram (simple):** OpenResponses items → Harmony conversation → token IDs → streamed markers → SSE. One box labeled "details in part 2".

---

## 7. Configuration and operability (practical section)

Readers will ask "how do I run it?" Cover:

- `uv pip install -r requirements-linux.txt` (CUDA llama-cpp-python wheel)
- `tiles run` flags: `--context-length`, `--gpu-layers`, `--offload-kqv`, `--batch-size` (PR #160)
- Settings persist under `[llama]` in `config.toml`
- Model reload when llama config changes even if model path is the same
- Logs: `~/.local/share/tiles/logs/server.out.log` (and daemon logs)

**Optional anecdote:** partial HuggingFace download → load fails → resume download retry loop in `repl.rs`.

---

## 8. Challenges worth naming (without solving them in depth)

Short honest section—good for credibility:

- **Parity pressure** — MLX and llama.cpp don't behave identically; `LlamaRunner` exists to absorb differences behind one interface.
- **Harmony is its own format** — gpt-oss isn't "just chat completion"; Harmony encoding is a separate path from standard GGUF chat templates.
- **Streaming vs batch** — streaming path got more testing; batch Harmony exists but is secondary.
- **Tool-call streaming** — `linux.py` state machine maps markers to OpenResponses SSE; edge cases around tool name normalization (`functions.read`, channel leakage).
- **Packaging** — bundling Python server + CUDA deps + Pi binary in the Linux installer (mention installer work from PR #138 without full detail).

Pick 2–3 you have personal stories for.

---

## 9. What we intentionally did not do

Helps readers understand scope:

- Did not embed llama.cpp in Rust—Python + llama-cpp-python keeps parity with existing server architecture.
- Did not make Pi platform-aware—provider config is the only bridge.
- Did not unify `linux.py` and `mlx.py` into one file yet (noted TODO in code); shared logic lives in `commons.py`.
- Did not expose raw llama.cpp C API—everything goes through llama-cpp-python.

---

## 10. Results and validation

Light section:

- Same REPL UX on Linux as Mac for the default gpt-oss flow
- Tests: `test_llama_cpp_runner.py`, `test_linux_streaming.py`, `test_commons.py` for Harmony conversation replay
- CHANGELOG: PR #138 (Linux support), PR #160 (llama config flags)

Optional: one benchmark or subjective note (tokens/s, first-token latency)—only if you have numbers.

---

## 11. Close — teaser for part 2

End with:

> This post was about *where* llama.cpp sits in Tiles. The next post goes inside `LlamaRunner`: CUDA preload, GGUF discovery, stop tokens, Harmony `StreamableParser`, and how we keep MLX and Linux streaming output aligned.

**Suggested part 2 title:** "Inside LlamaRunner: Harmony, stop tokens, and MLX parity on Linux"

---

## Diagram checklist (for you to draw)

| # | Diagram | Purpose |
|---|---------|---------|
| 1 | macOS vs Linux stack (shared middle) | Motivation |
| 2 | Four processes + ports | Mental model |
| 3 | Startup sequence | First-run flow |
| 4 | One-prompt sequence (REPL → Pi → server → runner) | Main narrative |
| 5 | Unchanged vs changed layers | Platform split |
| 6 | gpt-oss path (shallow) | Harmony mention |
| 7 | Marker yield flow (optional, or save for part 2) | Streaming semantics |

---

## Code references worth citing in the article

Link or cite these files—readers like anchors:

| File | Why |
|------|-----|
| `server/main.py` | `get_backend()` platform switch |
| `server/api.py` | `/start`, `/v1/responses` |
| `tiles/src/repl.rs` | Server boot, model load, Pi RPC, REPL loop |
| `tiles/src/daemon.rs` | Model cache path for Python server |
| `server/backend/linux.py` | Linux orchestration + streaming |
| `server/backend/commons.py` | Harmony + SSE shared with MLX |
| `server/backend/llama_cpp_runner.py` | Runner (intro only in this post) |
| `tiles/src/utils/config.rs` | Pi `models.json`, `[llama]` config |

---

## Length and structure suggestion

- **Target:** 1,500–2,500 words
- **Sections 1–4:** ~60% of the post (story + flow)
- **Sections 5–7:** ~30% (platform split + config)
- **Sections 8–11:** ~10% (challenges, scope, teaser)

---

## Disclosure reminder

Per project `AGENTS.md` / `CONTRIBUTING.md`: if this ships as an official Tiles post and AI helped with drafting, disclose meaningful AI usage in the PR or post footer.
