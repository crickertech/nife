# Renting an open-weight model for the mechanical lanes

**Provisional name** (`notes/open-model-lanes.md`). Written 2026-09-22, when calef said he would run
out of subscription tokens the following day and asked for an open model to replace the cheaper
Claude tier.

**The decisions already existed and nobody had built them.** §202 (mechanical work goes to a cheaper
model) routes the work; §203 (capacity is rented rather than bought) rules that the capacity is
rented, prices the mechanical load at roughly **2 million tokens a day**, and says the binding
constraint out loud: *"calef's limit is a rate limit rather than a bill"*. On 2026-09-21, twenty
lanes spent about **5 million tokens** and **6% of them ran anywhere other than the expensive
model**. This note is the missing mechanism, not a new ruling.

## Why nothing is hosted locally

Re-measured 2026-09-22 and unchanged from §203's figures:

| machine | what it is | why not |
|---|---|---|
| **patagonia** | Apple M3, 8 cores, **16 GB** | it is already the ceiling: `VERIFY_JOBS` is 4, two lanes gating at once has killed a session, and the lane count tops out at three or four. Inference would compete for the memory the gates need |
| **cordoba** | 2013 i5-4670, 4 cores, 23 GB with **3.6 GB free**, GT 1030 | Immich holds 15 GB serving the family's photos. It is doing a real job well |

So: **rent**, which is what §203 ruled.

## The shape, and the one fact that forces it

**One `claude` process talks to exactly one provider.** The endpoint is resolved at startup, and a
subagent's `model` field accepts only Claude aliases (`sonnet`, `opus`, `haiku`, `fable`, `best`,
`inherit`, or a full Claude id). There is no per-subagent endpoint, in frontmatter or in settings.

So a session cannot send *some* of its lanes elsewhere. A mechanical lane has to be a **separate
headless process**, which is what `helpers/open-lane.sh` runs:

```
cd <worktree> && ANTHROPIC_BASE_URL=<gateway> ANTHROPIC_AUTH_TOKEN=<token> \
    ANTHROPIC_MODEL=<id> claude --bare -p "<brief>" --allowedTools "Bash,Read,Edit,Write,Glob,Grep"
```

**The endpoint must speak the Anthropic Messages API** (`POST /v1/messages`). There is no
OpenAI-compatible client mode. Open-weight providers are OpenAI-shaped, so a translating gateway
sits in between; the Claude Code documentation names **LiteLLM** and **Kong**.

## The provider is OpenRouter, and it needs the gateway

calef chose OpenRouter on 2026-09-22. Checked against its own API reference the same day: it
exposes **`POST /api/v1/chat/completions`** with `Authorization: Bearer`, describes itself as
OpenAI-compatible, and **publishes no `/v1/messages`**. So the gateway is not optional here, and it
is the documented shape rather than a workaround: LiteLLM's proxy serves `/v1/messages` in Anthropic
format and translates to an OpenAI-compatible upstream.

```
Claude Code  --/v1/messages-->  LiteLLM (127.0.0.1:4000)  --/chat/completions-->  OpenRouter
```

`config/open-lane-litellm.yaml` holds the mapping and `helpers/open-lane-gateway.sh` starts it.

**It runs on cordoba, not on patagonia** (calef, 2026-09-22, wanting to call it from several hosts
on his tailnet). Three reasons beyond that one. cordoba is **always on**, where a laptop is not, and
a gateway that sleeps with the lid leaves callers failing on a connection error rather than a gate
failure. It keeps the **OpenRouter key on one machine** instead of copied to each. And it spends
nothing that matters: LiteLLM runs no model, so against cordoba's 3.6 GB free it is a translator
rather than a load, while patagonia's 16 GB is the thing that actually caps the lane count.
`config/open-lane-gateway.service` is the unit.

**It never binds a network interface, and Tailscale does the exposing** (calef's question,
2026-09-22: can the port be reachable only over the tailnet). It can, and the best form of that is
not a firewall rule:

```
tailscale serve --bg --https=4000 http://127.0.0.1:4000
```

`tailscale serve` is **tailnet-only by definition**, which its own documentation states and
contrasts with `tailscale funnel`, the command that publishes to the public internet. **Funnel must
never be used for this.** Serve also provisions TLS, so callers reach
`https://<host>.<tailnet>.ts.net:4000` rather than sending an API key over plaintext.

**Three layers, and no gateway password** (calef, 2026-09-22: *"I already have a spend limit. Lets
remove the key."*).

| layer | what it decides |
|---|---|
| loopback bind | only cordoba's own processes can open the socket at all |
| `tailscale serve` | the tailnet, and nothing else, reaches it |
| Tailscale ACLs | *which* tailnet nodes reach it, and this is the authorization |

The first matters most, because it is the only one that survives a later mistake: an interface that
appears later, or a firewall rule edited wrongly, cannot reach a socket that was never bound.
Binding `OPEN_LANE_HOST=<tailnet address>` is supported and is second best; `0.0.0.0` is refused
outright.

**What this accepts, recorded because it is a choice rather than an oversight.** LiteLLM listens on
cordoba's loopback with no credential, so **any process on cordoba can spend the OpenRouter key**,
and cordoba runs Immich. The tailnet cannot help there, because that traffic never crosses it. The
backstop is the **spend limit on the OpenRouter key itself**, which calef had already set and which
is the only control here that still works after a key has leaked. A gateway password would have
narrowed the loopback case and nothing else; it was weighed and declined.

## What makes a cheaper model safe here, and it is not the model

**The gates are the oracle.** `helpers/open-lane.sh` never judges the work: it loops the model
against `script/lint` and `script/citations --ratchet`, feeds the failing output back as the next
round's prompt, and hands the worktree back unmerged if it cannot reach green inside a round budget.
That is the same argument §202 makes, mechanised: a cheaper model is safe exactly to the extent that
a shell command says pass or fail.

**So the routing rule is about the oracle, not about difficulty.** Work with a crisp gate goes to
the open model: bisections (the gate's exit code *is* the answer), reference sweeps, renumbering,
promotions, formatting, mechanical repairs. Work whose output is a judgement stays on Claude: design
forks, anything touching the syscall surface or a wire format, prose a reader will later trust, and
adversarial passes. Yesterday's findings that mattered most, `size_of::<PerCpu>()` breaking a shift,
a capability surviving a revocation sweep, a calibration wrong by 11x, all came from the second kind.

## Maintainer work is the best target found, and the reason is not the model

**Measured 2026-09-22.** A rebase with three known conflict classes, handed to Qwen3-Coder through
`helpers/open-lane.sh`, cost **$0.0552**, was correct first time, needed no redo, and replaced about
thirteen of the maintainer's tool calls with four. It took `main`'s baselines rather than
hand-merging them, which is the trap that nearly shipped a wrong benchmark floor twice that evening.

**It worked because the brief encoded judgement that had already been made**, not because the model
is clever: three named resolutions, and an instruction to abort on anything else. That is lookup
rather than judgement, and lookup is what a cheap model is good at.

So the briefs live in `briefs/` as checked-in assets rather than being retyped from memory, because
a brief written fresh each time loses a clause a month and the clause it loses is the one that stops
a wrong conflict resolution shipping as housekeeping.

**This reframes the offload estimate.** The decision that the subscription stays and rented models
fill the mechanical tail, taken the same day and not yet on `main`, puts the lane tail at about 18%
of a day's tokens. Maintainer work sits on top of
that, is nearly all delegatable at these prices, and is done in the most expensive context
available. Nobody has measured it, because the session's own consumption is not instrumented. The milestone for
that, *what a lane spent on its milestone*, was promoted the same day and is not yet on `main`, so
it is named here rather than cited.

## What has to be benchmarked before this is trusted

**Tool-call fidelity, and nothing in the documentation vouches for it.** Claude Code sends tool
definitions in Anthropic's schema; the gateway translates them; whether a given open-weight model
emits well-formed calls turn after turn is a property of that model. The honest first step is one
real mechanical task, run against the candidate model, scored on: did it edit the right files, did
it run the gate, did it read the exit code, and how many rounds did green take.

## BUGS

- **Prompt caching is billed as a miss.** Claude Code sends `cache_control` regardless of the
  upstream; a gateway that does not implement it bills every turn uncached. §203's per-token
  estimate assumed no caching, so it stands, but any quote at a cached rate is wrong.
- **The context window is guessed.** For a model id Claude Code does not recognise it assumes 200K.
  Set `CLAUDE_CODE_MAX_CONTEXT_TOKENS` if the real window is smaller, or a run truncates mid-task.
- **`--bare` skips `AGENTS.md`, skills, hooks and plugins.** Deliberate: the constitution is 924
  lines and a cheap model would spend its window on them. The cost is that an open-model lane does
  not inherit the rules, so its brief must carry everything it needs, including that citations need
  glosses and that a lane never edits another milestone's block.
- **Beta request fields can hard-fail.** Claude Code sends Anthropic-specific fields to whatever
  `ANTHROPIC_BASE_URL` names. `helpers/open-lane.sh` sets `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1`
  for that reason; a gateway that strips them itself would also do.
- **Nobody has run this yet.** It is written from the documentation and from measured hardware, and
  it has not driven a single lane. The first run is the benchmark above, and until it happens this
  note describes an intention.
- **`OPEN_LANE_EFFORT` exists and is unmeasured against this mechanism specifically.**
  `helpers/open-lane.sh` now passes `claude --effort`, defaulting to `low`
  ([effort-levels.md](effort-levels.md)), but that default was measured against Claude directly, not
  through this gateway against an open-weight model. Whether the flag reaches the model at all once
  `config/open-lane-litellm.yaml`'s `drop_params: true` has a chance to strip it is exactly the kind
  of thing "nobody has run this yet" above already flags; this is the same gap, one layer more
  specific.
