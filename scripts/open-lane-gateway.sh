#!/bin/sh
# Start the LiteLLM gateway that `scripts/open-lane.sh` talks to.
#
#     export OPENROUTER_API_KEY=sk-or-...
#     export OPEN_LANE_TOKEN=$(head -c 24 /dev/urandom | base64)   # any secret; the proxy's own
#     scripts/open-lane-gateway.sh
#
# **Why a gateway at all.** Claude Code resolves its endpoint at startup and speaks the Anthropic
# Messages API (`POST /v1/messages`); OpenRouter speaks OpenAI chat-completions and exposes no
# `/v1/messages`, confirmed against its own API reference on 2026-09-22. LiteLLM exposes the former
# and translates to the latter. It runs no model, so it is a translator rather than a load.
#
# **Loopback only.** This process holds an OpenRouter key. It binds 127.0.0.1 and is not to be
# exposed; the `master_key` is what stops another local process using the key by accident, not a
# defence against anything that already has the machine.
#
# # BUGS
#
# - **It has to be running before a lane starts**, and nothing supervises it. A lane whose gateway
#   is down fails its first round with a connection error rather than a gate failure, which reads
#   confusingly in `open-lane.sh`'s output.
# - **`uvx` fetches LiteLLM on first run**, so the first start is slow and needs the network.
set -eu

: "${OPENROUTER_API_KEY:?set OPENROUTER_API_KEY (https://openrouter.ai/keys)}"
: "${OPEN_LANE_TOKEN:?set OPEN_LANE_TOKEN to any secret; the gateway requires it from callers}"

port=${OPEN_LANE_PORT:-4000}
config=$(dirname "$0")/../config/open-lane-litellm.yaml

echo "==> LiteLLM on 127.0.0.1:$port, translating /v1/messages to OpenRouter"
echo "==> then: export OPEN_LANE_BASE_URL=http://127.0.0.1:$port"
exec uvx --from 'litellm[proxy]' litellm \
    --config "$config" --host 127.0.0.1 --port "$port"
