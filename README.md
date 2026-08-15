# Smallville — Generative Agents

A Smallville-style **generative-agents experiment** inspired by the Stanford
paper *"Generative Agents: Interactive Simulacra of Human Behavior"* (Park et
al., 2023). Ten LLM-driven residents live out their days in a scripted town
with **memory streams**, **reflection**, and **daily planning** — watch them
form routines, strike up friendships, and quietly build a civilization.

Rust simulation engine + live browser visualization. No game; an experiment in
observing emergent social behavior.

## Quick start

```bash
cargo run                      # needs OPENAI_API_KEY (any OpenAI-compatible API)
```

Then open **http://localhost:8080**.

### Run without an API key (scripted mock town)

```bash
SIM_MOCK=1 cargo run
```

`SIM_MOCK=1` makes the LLM client return scripted plans/dialogue/reflections so
you can demo the full sim — movement, conversations, and memory — offline.

### Configuration

| Env / flag | Default | Purpose |
| --- | --- | --- |
| `OPENAI_API_KEY` | — | API key (or `RUSTY_LLM_API_KEY`) |
| `SIM_MODEL` / `--model` | `gpt-4o-mini` | chat model for plans, dialogue, reflection |
| `SIM_EMBED_MODEL` / `--embed-model` | `text-embedding-3-small` | embeddings for memory retrieval |
| `SIM_BASE_URL` / `--base-url` | `https://api.openai.com/v1` | OpenAI-compatible endpoint |
| `--hour-seconds` | `8` | real seconds per in-game hour |
| `--port` | `8080` | web UI port |

```bash
export OPENAI_API_KEY=sk-...
cargo run -- --hour-seconds 5 --port 9000
```

## The experiment

The town: a family house, Hobbit Hole Café, Willow Market, a park, a library,
and City Hall. Ten residents with distinct bios, traits, homes, and workplaces.

Each in-game hour (default 8 real seconds):

1. **Morning planning** — every agent LLM-generates its full daily schedule
   (`08:00|LOCATION|action`), grounded in its bio and the town's locations.
2. **Action** — agents execute the hour's plan, moving between locations and
   recording the action into their memory stream.
3. **Conversation** — when two or more agents share a location, the LLM writes
   a short dialogue between them, retrieved memories in hand. Lines are stored
   as memories and each pair's **relationship score** ticks up.
4. **Reflection** — every four hours, agents with enough experience generate
   higher-level insights about themselves and the town, stored alongside
   observations.
5. **Daily chronicle** — at 22:00 the LLM summarizes the town's day.

### Memory retrieval (from the paper)

Every memory entry scores on a blend of:

- **Recency** — exponential decay `0.98^hours`, so fresh memories win,
- **Importance** — a 1–10 score from a keyword heuristic,
- **Relevance** — cosine similarity of embeddings (falls back to token
  overlap offline).

The top-k retrieved memories feed the LLM when an agent speaks or reflects —
so an agent's *past* shapes its *present*, which is where the emergent
behavior comes from.

### What to watch for

- Agents converging on the café at lunch and striking up conversations
- Relationship scores forming (click a resident to see theirs)
- Reflections that mention *other agents* by name
- Daily summaries that change as the town develops

## Architecture

```
src/
├── main.rs     entry: spawns the sim thread + async HTTP server
├── sim.rs      the engine: plans, movement, dialogue, reflection, summaries
├── agent.rs    Agent + daily PlanEntry
├── memory.rs   MemoryStream + retrieval scoring (recency/importance/relevance)
├── llm.rs      OpenAI-compatible chat + embeddings client, SIM_MOCK mode
├── town.rs     locations + fuzzy name lookup
├── state.rs    shared snapshot + event ring buffer (shared with the server)
├── server.rs   axum API: /api/state, /api/events, static web/ files
└── config.rs   CLI + env configuration

web/
├── index.html  town map, event feed, resident inspector
├── style.css   dark, monospace, amber-accented
└── app.js      polling frontend (map + live feed + detail panel)
```

The sim runs on a dedicated thread and publishes to a lock-protected
`SharedState`; the axum server reads it. Polling frontend (1.5 s) with smooth
CSS transitions for movement.

## API

| Endpoint | Returns |
| --- | --- |
| `GET /api/state` | day, clock, locations, all agents (location, action, plan, memories, relationships) |
| `GET /api/events?after=<id>` | events newer than `<id>` (move, act, dialogue, reflection, summary) |

## Tests

```bash
cargo test
```

## Roadmap

- LLM-scored importance (off by default to save tokens)
- Memory retrieval hyperparameters via CLI
- Agent-driven events (a resident *throws* a party, others attend)
- Persistent town state across restarts
- Relationship-driven dialogue (friends chat differently than strangers)
