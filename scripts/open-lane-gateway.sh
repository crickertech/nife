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
# **Where it binds decides what the master key is for**, and the two cases are different enough to
# be worth stating rather than discovered.
#
#   - **Loopback** (`OPEN_LANE_HOST=127.0.0.1`, the default). The `master_key` guards against
#     another local process using the OpenRouter key by accident. It is not a defence against
#     anything that already has the machine.
#   - **A tailnet address**, which is why this runs on cordoba: one always-on host holds the key and
#     every machine calls it. Now the `master_key` is **the only thing between any host on that
#     network and the OpenRouter bill**. Give it real entropy, keep it out of the repository, and
#     bind to the tailnet interface rather than `0.0.0.0`, so a future coffee-shop network cannot
#     reach it. Tailscale ACLs are the second layer and are worth setting.
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
host=${OPEN_LANE_HOST:-127.0.0.1}
config=$(dirname "$0")/../config/open-lane-litellm.yaml

case $host in
    127.0.0.1|localhost) ;;
    0.0.0.0) echo >&2 "open-lane-gateway: refusing 0.0.0.0. Bind the tailnet address, so a network"
             echo >&2 "open-lane-gateway: this machine joins later cannot reach the OpenRouter key."
             exit 2 ;;
    *) echo "==> binding $host: the master key is now the only thing guarding the OpenRouter key" ;;
esac

echo "==> LiteLLM on $host:$port, translating /v1/messages to OpenRouter"
echo "==> then, on any caller: export OPEN_LANE_BASE_URL=http://$host:$port"
exec uvx --from 'litellm[proxy]' litellm \
    --config "$config" --host "$host" --port "$port"
