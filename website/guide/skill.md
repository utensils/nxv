# Agent Skill

nxv ships an [Agent Skills](https://agentskills.io)-standard skill that lets AI
coding agents — Claude Code, OpenAI Codex CLI, Pi, OpenClaw, GitHub Copilot CLI,
Cursor, Gemini CLI, Amp, Goose, and anything else that reads `SKILL.md` — run
nxv commands on your behalf without extra setup. The nxv binary embeds the skill
and installs it for you.

## What is a skill?

A skill is a `SKILL.md` file (with optional supporting files) that teaches an
agent how to use a tool. When the skill is loaded, the agent knows every
subcommand, flag, JSON output shape, and HTTP endpoint for nxv. It can answer
questions like "which nixpkgs commit shipped python 2.7?" or "give me the
`nix shell` command for nodejs 15" by running the right `nxv` invocation (or
hitting the public API) and interpreting the result.

## Installing the skill

The `nxv skill` subcommand generates and installs the skill from a single
embedded template:

```bash
# Install user-wide for every agent detected on this machine
nxv skill install

# Install into the current project (.claude/skills + .agents/skills,
# which every supported agent reads at project level)
nxv skill install --project

# Install for specific agents only
nxv skill install claude codex

# Install for every supported agent, detected or not
nxv skill install --all

# See supported agents, their paths, and what's installed where
nxv skill list
```

A user-wide install (the default) targets agents whose config directory exists
under your home directory. Per agent, the skill lands in
`<skills dir>/nxv/SKILL.md`:

| Agent      | User-wide                 | Project-level     |
| ---------- | ------------------------- | ----------------- |
| `claude`   | `~/.claude/skills/`       | `.claude/skills/` |
| `codex`    | `~/.codex/skills/`        | `.agents/skills/` |
| `pi`       | `~/.pi/agent/skills/`     | `.pi/skills/`     |
| `openclaw` | `~/.openclaw/skills/`     | `.agents/skills/` |
| `copilot`  | `~/.copilot/skills/`      | `.github/skills/` |
| `cursor`   | `~/.cursor/skills/`       | `.agents/skills/` |
| `gemini`   | `~/.gemini/skills/`       | `.agents/skills/` |
| `amp`      | `~/.config/amp/skills/`   | `.agents/skills/` |
| `goose`    | `~/.config/goose/skills/` | `.agents/skills/` |
| `agents`   | `~/.agents/skills/`       | `.agents/skills/` |

The `agents` target is the generic cross-agent directory from the Agent Skills
standard — most tools read it, so it also serves as the fallback when no agents
are detected.

The table shows each agent's primary directory — the one
`nxv skill install <agent>` writes to. Several agents read additional locations:
Copilot reads `.github/skills/`, `.claude/skills/`, or `.agents/skills/` in a
repository, and Pi reads `.agents/skills/` as well as `.pi/skills/`. That is why
the default project install writes only the `.claude` + `.agents` pair: every
supported agent picks up one of the two.

To remove installed skills:

```bash
nxv skill uninstall            # Remove from every user-wide agent path
nxv skill uninstall --project  # Remove project-level installs
```

### Manual install (no nxv binary)

Fetch the canonical copy straight from the repository:

```bash
mkdir -p ~/.claude/skills/nxv
curl -sL https://raw.githubusercontent.com/utensils/nxv/main/.claude/skills/nxv/SKILL.md \
  -o ~/.claude/skills/nxv/SKILL.md
```

## Using the skill

Once installed, agents that support slash-command invocation can call it
directly:

```
/nxv search python 2.7
/nxv search python 2.7.3 --all-depths
/nxv info python311 3.11.4
/nxv history nodejs_15
```

Or just ask naturally — the agent loads the skill automatically when your
question matches its description:

> "Which nixpkgs commit had python 2.7?"
>
> "Give me the `nix shell` command for nodejs 15.14."
>
> "When was ruby 2.6 last in nixpkgs?"

You don't need a local index for the skill to be useful — agents can hit the
public API at `https://nxv.urandom.io` directly, or you can set
`NXV_API_URL=https://nxv.urandom.io` in your environment so the CLI uses the
hosted instance transparently.

## For agents

The skill is designed so autonomous agents can extract structured data reliably.
The `search`, `info`, and `history` subcommands support `--format json`, and
every data-returning HTTP API response is wrapped in a stable `{ "data": ... }`
envelope (plus `meta` for paginated lists; only the operational `/health` and
`/metrics` endpoints are unwrapped). Agents should:

1. Run `nxv <subcommand> --format json` (CLI) or hit `/api/v1/...` (HTTP) for
   machine-readable output.
2. Pipe to `jq` (or parse in-process) for the specific field they need.
3. Never rely on the human-readable table output — column widths and formatting
   are terminal-dependent.

For a version-qualified prefix search, nxv searches the shallowest matching
attribute-path tier first. If no version matches, API consumers should inspect
`meta.resolution.suggestions` and `deeper_matches_available`; pass
`all_depths=true` only when nested package-set matches are intentional.
Successful CLI JSON searches return an array; an empty miss emits no stdout,
with the miss explanation written to stderr.

Example agent pattern — generate a `nix shell` invocation for a specific version
directly from the public API:

```bash
curl -s "https://nxv.urandom.io/api/v1/packages/python27/versions/2.7.18/first" | \
  jq -r '.data | "nix shell nixpkgs/\(.first_commit_hash | .[0:7])#\(.attribute_path)"'
```

Or via the CLI against any backend:

```bash
nxv search nodejs 15 --exact --format json | \
  jq -r '.[0] | "nix shell nixpkgs/\(.first_commit_hash | .[0:7])#\(.attribute_path)"'
```

## Keeping the skill up to date

The installed skill is byte-identical to the template embedded in your nxv
binary, so refreshing it is just upgrading nxv and reinstalling:

```bash
nxv update           # Get the latest nxv (also refreshes the index)
nxv skill install    # Rewrite the installed skills from the new binary
```

## Skill source

The canonical template lives at
[`src/skill/SKILL.md`](https://github.com/utensils/nxv/blob/main/src/skill/SKILL.md)
in the repository; the checked-in copies under `.claude/skills/` and
`.agents/skills/` are generated from it.
