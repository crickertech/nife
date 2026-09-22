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
headless process**, which is what `scripts/open-lane.sh` runs:

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

`config/open-lane-litellm.yaml` holds the mapping and `scripts/open-lane-gateway.sh` starts it.

**It runs on cordoba, not on patagonia** (calef, 2026-09-22, wanting to call it from several hosts
on his tailnet). Three reasons beyond that one. cordoba is **always on**, where a laptop is not, and
a gateway that sleeps with the lid leaves callers failing on a connection error rather than a gate
failure. It keeps the **OpenRouter key on one machine** instead of copied to each. And it spends
nothing that matters: LiteLLM runs no model, so against cordoba's 3.6 GB free it is a translator
rather than a load, while patagonia's 16 GB is the thing that actually caps the lane count.
`config/open-lane-gateway.service` is the unit.

**Leaving loopback changes what the master key is for**, which is worth stating rather than
discovering. On `127.0.0.1` it guards against another local process using the OpenRouter key by
accident. On a tailnet address it is **the only thing between any host on that network and the
OpenRouter bill**. So: real entropy, kept in `/etc` rather than in this repository, bound to the
tailnet interface rather than `0.0.0.0` (the launcher refuses `0.0.0.0` for that reason), with
Tailscale ACLs as the second layer.

**Why OpenRouter rather than a provider directly**, stated so the next person does not re-litigate
it: one account and one key reach every open-weight model, so switching candidates is a line in a
config rather than a new signup, and that is exactly what an unmeasured choice needs. The cost is a
margin on top of the underlying provider's price and one more party in the path.

**The model is not chosen.** `config/open-lane-litellm.yaml` carries three candidates addressable by
name so a benchmark can switch between them without editing the lane script. Nothing here has
measured which of them drives a tool loop reliably, and the config says so.

## What makes a cheaper model safe here, and it is not the model

**The gates are the oracle.** `scripts/open-lane.sh` never judges the work: it loops the model
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
  `ANTHROPIC_BASE_URL` names. `scripts/open-lane.sh` sets `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1`
  for that reason; a gateway that strips them itself would also do.
- **Nobody has run this yet.** It is written from the documentation and from measured hardware, and
  it has not driven a single lane. The first run is the benchmark above, and until it happens this
  note describes an intention.
