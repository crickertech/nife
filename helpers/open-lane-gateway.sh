#!/bin/sh
# Start the LiteLLM gateway that `scripts/open-lane.sh` talks to.
#
#     export OPENROUTER_API_KEY=sk-or-...        # issued by OpenRouter, stays on this machine
#     scripts/open-lane-gateway.sh
#
# **There is no gateway password, deliberately** (calef, 2026-09-22). The tailnet is the boundary
# and the OpenRouter key carries a spend limit, so a second secret would guard little and cost a
# thing to remember. What that accepts is written in notes/open-model-lanes.md rather than left to
# be discovered.
#
# **It stays on loopback, and Tailscale does the exposing.** The gateway holds an OpenRouter key,
# so the strongest thing available is for it never to bind a network interface: a firewall rule that
# is wrong later, or an interface that appears later, then cannot reach it at all. On cordoba:
#
#     tailscale serve --bg --https=4000 http://127.0.0.1:4000
#
# `tailscale serve` is **tailnet-only** by definition, which its own documentation states in as many
# words and contrasts with `tailscale funnel`, the one that publishes to the internet. **Never use
# funnel for this.** Serve also provisions TLS, so callers get `https://<host>.<tailnet>.ts.net:4000`
# rather than plaintext over the tailnet.
#
# **Three layers, and each does a different job.** Loopback binding means only this machine's own
# processes can open the socket. `tailscale serve` decides that the tailnet, and nothing else,
# reaches it. Tailscale ACLs decide *which* tailnet nodes. The `master_key` is then the last one, and
# guards against a tailnet node that is allowed to connect but should not be spending the key.
#
# **Binding a real address is still supported and is second best.** `OPEN_LANE_HOST=<tailnet addr>`
# works, and then the master key is doing more of the work. `0.0.0.0` is refused outright.
#
# # BUGS
#
# - **It has to be running before a lane starts**, and nothing supervises it. A lane whose gateway
#   is down fails its first round with a connection error rather than a gate failure, which reads
#   confusingly in `open-lane.sh`'s output.
# - **`uvx` fetches LiteLLM on first run**, so the first start is slow and needs the network.
set -eu

: "${OPENROUTER_API_KEY:?set OPENROUTER_API_KEY (https://openrouter.ai/keys)}"

port=${OPEN_LANE_PORT:-4000}
host=${OPEN_LANE_HOST:-127.0.0.1}
config=$(dirname "$0")/../config/open-lane-litellm.yaml

case $host in
    127.0.0.1|localhost) ;;
    0.0.0.0) echo >&2 "open-lane-gateway: refusing 0.0.0.0. Bind the tailnet address, so a network"
             echo >&2 "open-lane-gateway: this machine joins later cannot reach the OpenRouter key."
             exit 2 ;;
    *) echo "==> binding $host rather than loopback. Prefer:"
       echo "==>   OPEN_LANE_HOST=127.0.0.1 $0"
       echo "==>   tailscale serve --bg --https=$port http://127.0.0.1:$port"
       echo "==> which keeps the socket off every interface and lets the tailnet reach it anyway." ;;
esac

echo "==> LiteLLM on $host:$port, translating /v1/messages to OpenRouter"
if [ "$host" = 127.0.0.1 ] || [ "$host" = localhost ]; then
    echo "==> expose it to the tailnet and nothing else, in another shell:"
    echo "==>   tailscale serve --bg --https=$port http://127.0.0.1:$port"
    echo "==> then, on any caller:"
    echo "==>   export OPEN_LANE_BASE_URL=https://<this-host>.<tailnet>.ts.net:$port"
else
    echo "==> then, on any caller: export OPEN_LANE_BASE_URL=http://$host:$port"
fi
exec uvx --from 'litellm[proxy]' litellm \
    --config "$config" --host "$host" --port "$port"
