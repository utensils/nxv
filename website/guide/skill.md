# Claude Code Skill

nxv ships a [Claude Code skill](https://code.claude.com/docs/en/skills) that
lets Claude — and agents like [openclaw](https://github.com/utensils/openclaw) —
run nxv commands on your behalf without extra setup.

## What is a skill?

A Claude Code skill is a `SKILL.md` file (with optional supporting files) that
teaches Claude how to use a tool. When the skill is loaded, Claude knows every
subcommand, flag, JSON output shape, and HTTP endpoint for nxv. It can answer
questions like "which nixpkgs commit shipped python 2.7?" or "give me the
`nix shell` command for nodejs 15" by running the right `nxv` invocation (or
hitting the public API) and interpreting the result.

The skill follows the open [Agent Skills](https://agentskills.io) standard, so
it works with Claude Code, the Claude Agent SDK, openclaw, and any other tool
that consumes the same `SKILL.md` format.

## Installing the skill

### Personal skill (all your projects)

```bash
mkdir -p ~/.claude/skills/nxv
curl -sL https://raw.githubusercontent.com/utensils/nxv/main/.claude/skills/nxv/SKILL.md \
  -o ~/.claude/skills/nxv/SKILL.md
```

This makes the skill available in every Claude Code session on your machine.

### Project-local skill

```bash
mkdir -p .claude/skills/nxv
curl -sL https://raw.githubusercontent.com/utensils/nxv/main/.claude/skills/nxv/SKILL.md \
  -o .claude/skills/nxv/SKILL.md
```

Only active when you're in that project's directory.

### From this repo (if you cloned nxv)

The skill is already at `.claude/skills/nxv/SKILL.md` in the repo root. Copy it
wherever you need it:

```bash
cp .claude/skills/nxv/SKILL.md ~/.claude/skills/nxv/SKILL.md
```

## Using the skill

Once installed, you can invoke it directly:

```
/nxv search python 2.7
/nxv info python311 3.11.4
/nxv history nodejs_15
```

Or just ask Claude naturally — it will load the skill automatically when your
question matches the description:

> "Which nixpkgs commit had python 2.7?"
>
> "Give me the `nix shell` command for nodejs 15.14."
>
> "When was ruby 2.6 last in nixpkgs?"

You don't need a local index for the skill to be useful — Claude can hit the
public API at `https://nxv.urandom.io` directly, or you can set
`NXV_API_URL=https://nxv.urandom.io` in your environment so the CLI uses the
hosted instance transparently.

## For agents (openclaw and others)

The skill is designed so autonomous agents can extract structured data reliably.
Every CLI subcommand that produces output supports `--format json`, and every
HTTP API response is wrapped in a stable `{ "data": ..., "meta": {...} }`
envelope. Agents should:

1. Run `nxv <subcommand> --format json` (CLI) or hit `/api/v1/...` (HTTP) for
   machine-readable output.
2. Pipe to `jq` (or parse in-process) for the specific field they need.
3. Never rely on the human-readable table output — column widths and formatting
   are terminal-dependent.

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

Pull the latest version any time nxv ships new subcommands, flags, or API
endpoints:

```bash
curl -sL https://raw.githubusercontent.com/utensils/nxv/main/.claude/skills/nxv/SKILL.md \
  -o ~/.claude/skills/nxv/SKILL.md
```

The skill file itself contains a self-update reminder at the bottom with this
same command.

## Skill source

The canonical skill lives at
[`.claude/skills/nxv/SKILL.md`](https://github.com/utensils/nxv/blob/main/.claude/skills/nxv/SKILL.md)
in the repository.
