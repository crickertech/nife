#!/bin/sh
# A Claude Code UserPromptSubmit hook: every time calef sends a prompt, put a short reminder in front
# of the session that it is a live conversation, so multi-step work goes to a background agent and
# the reply comes first. Claude Code adds a UserPromptSubmit hook's plain stdout to the context.
#
# Why this exists, and why at this rung. AGENTS.md's top-up rule already says it: "A conversation
# with calef never blocks the queue." On 2026-09-24 a maintainer session broke it repeatedly on a
# freshly cleared context, doing edits, gates and pull request plumbing in the foreground while
# calef waited for an answer. The rule was in context and lost on salience in an 11,000-word
# constitution. Prose is rung four; this is still rung four (a reminder, not a gate), but it is
# placed where the model decides how to act, on every prompt, instead of somewhere it read once.
#
# BUGS. It costs tokens on every prompt: about 50 words of context per message calef sends, for the
# life of every session in a nife checkout. It is a reminder, not a gate: nothing checks the reply,
# and it cannot stop foreground work that the model chooses anyway. It fires for every session in
# the tree, including one calef opens for a single quick question where dispatching would be silly;
# "more than a couple of tool calls" is the model's judgment to apply. Subagents never see it,
# because UserPromptSubmit fires only on a person's prompt, which is what a lane wants.
#
# Name: provisional. Minted 2026-09-24 by a lane (maintainer/prompt-reminder-hook); calef has not
# ratified it.
cat <<'REMINDER'
calef is waiting on this conversation. Answer him first. Any work taking more than a couple of tool calls goes to a background agent (Agent tool, run_in_background: true), and your reply says it was dispatched. Cheap reversible fixes still get done, dispatched rather than listed for him.
REMINDER
