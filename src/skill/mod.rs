//! Agent skill generation and installation.
//!
//! Embeds the canonical nxv SKILL.md (Agent Skills standard,
//! <https://agentskills.io>) and installs it into the skills directories of
//! supported AI coding agents, user-wide or per project. The installed file
//! is byte-identical to the embedded template; install always overwrites
//! `<skills dir>/nxv/SKILL.md` and never touches sibling files.

use anyhow::{Context, Result};
use clap::ValueEnum;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cli::{Cli, SkillInstallArgs, SkillListArgs, SkillUninstallArgs};

/// The embedded skill content, installed verbatim.
pub const SKILL_MD: &str = include_str!("SKILL.md");

/// Directory the skill is installed under: `<skills dir>/nxv/SKILL.md`.
pub const SKILL_DIR_NAME: &str = "nxv";

/// Supported AI coding agents.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Agent {
    /// Claude Code (~/.claude/skills, .claude/skills)
    Claude,
    /// OpenAI Codex CLI (~/.codex/skills, .agents/skills)
    Codex,
    /// Pi (~/.pi/agent/skills, .pi/skills)
    Pi,
    /// OpenClaw (~/.openclaw/skills, .agents/skills)
    Openclaw,
    /// GitHub Copilot CLI (~/.copilot/skills, .github/skills)
    Copilot,
    /// Cursor (~/.cursor/skills, .agents/skills)
    Cursor,
    /// Gemini CLI (~/.gemini/skills, .agents/skills)
    Gemini,
    /// Amp (~/.config/amp/skills, .agents/skills)
    Amp,
    /// Goose (~/.config/goose/skills, .agents/skills)
    Goose,
    /// Generic cross-agent directory (~/.agents/skills, .agents/skills)
    Agents,
}

/// Per-agent skill locations. Paths are stored as segments so joins are
/// correct on every platform.
pub struct AgentSpec {
    pub agent: Agent,
    /// CLI value name (clap kebab-case) — used in status messages.
    pub name: &'static str,
    /// Home-relative dir whose existence marks the agent as present.
    detect: &'static [&'static str],
    /// Home-relative skills parent dir (the skill goes in `<this>/nxv/`).
    user_skills: &'static [&'static str],
    /// Project-root-relative skills parent dir.
    project_skills: &'static [&'static str],
}

/// All supported agents, in CLI declaration order.
pub const AGENTS: &[AgentSpec] = &[
    AgentSpec {
        agent: Agent::Claude,
        name: "claude",
        detect: &[".claude"],
        user_skills: &[".claude", "skills"],
        project_skills: &[".claude", "skills"],
    },
    AgentSpec {
        agent: Agent::Codex,
        name: "codex",
        detect: &[".codex"],
        user_skills: &[".codex", "skills"],
        project_skills: &[".agents", "skills"],
    },
    AgentSpec {
        agent: Agent::Pi,
        name: "pi",
        detect: &[".pi"],
        user_skills: &[".pi", "agent", "skills"],
        project_skills: &[".pi", "skills"],
    },
    AgentSpec {
        agent: Agent::Openclaw,
        name: "openclaw",
        detect: &[".openclaw"],
        user_skills: &[".openclaw", "skills"],
        project_skills: &[".agents", "skills"],
    },
    AgentSpec {
        agent: Agent::Copilot,
        name: "copilot",
        detect: &[".copilot"],
        user_skills: &[".copilot", "skills"],
        project_skills: &[".github", "skills"],
    },
    AgentSpec {
        agent: Agent::Cursor,
        name: "cursor",
        detect: &[".cursor"],
        user_skills: &[".cursor", "skills"],
        project_skills: &[".agents", "skills"],
    },
    AgentSpec {
        agent: Agent::Gemini,
        name: "gemini",
        detect: &[".gemini"],
        user_skills: &[".gemini", "skills"],
        project_skills: &[".agents", "skills"],
    },
    AgentSpec {
        agent: Agent::Amp,
        name: "amp",
        detect: &[".config", "amp"],
        user_skills: &[".config", "amp", "skills"],
        project_skills: &[".agents", "skills"],
    },
    AgentSpec {
        agent: Agent::Goose,
        name: "goose",
        detect: &[".config", "goose"],
        user_skills: &[".config", "goose", "skills"],
        project_skills: &[".agents", "skills"],
    },
    AgentSpec {
        agent: Agent::Agents,
        name: "agents",
        detect: &[".agents"],
        user_skills: &[".agents", "skills"],
        project_skills: &[".agents", "skills"],
    },
];

impl Agent {
    /// Static spec for this agent.
    pub fn spec(&self) -> &'static AgentSpec {
        AGENTS
            .iter()
            .find(|s| s.agent == *self)
            .expect("every Agent variant has an AgentSpec entry")
    }

    /// User-wide skill directory (`<home>/<user_skills>/nxv`).
    pub fn user_skill_dir(&self, home: &Path) -> PathBuf {
        join_segments(home, self.spec().user_skills).join(SKILL_DIR_NAME)
    }

    /// Project-level skill directory (`<root>/<project_skills>/nxv`).
    pub fn project_skill_dir(&self, root: &Path) -> PathBuf {
        join_segments(root, self.spec().project_skills).join(SKILL_DIR_NAME)
    }

    /// Whether the agent appears to be present on this machine (its config
    /// directory exists under the home directory).
    pub fn is_detected(&self, home: &Path) -> bool {
        join_segments(home, self.spec().detect).exists()
    }
}

fn join_segments(base: &Path, segments: &[&str]) -> PathBuf {
    let mut path = base.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}

/// Install scope resolved from `--project` / `--dir`.
enum Scope {
    /// User-wide installs under the home directory.
    User(PathBuf),
    /// Project-level installs under the given root.
    Project(PathBuf),
}

/// Resolve the scope from the shared `--project` / `--dir` flags.
/// `--dir` implies a project install rooted at that directory.
fn resolve_scope(project: bool, dir: &Option<PathBuf>) -> Result<Scope> {
    if let Some(dir) = dir {
        Ok(Scope::Project(crate::paths::expand_tilde(dir)))
    } else if project {
        Ok(Scope::Project(
            std::env::current_dir().context("could not determine the current directory")?,
        ))
    } else {
        dirs::home_dir().map(Scope::User).ok_or_else(|| {
            anyhow::anyhow!(
                "could not determine the home directory (is HOME set?); \
                 use --project or --dir for a project-level install"
            )
        })
    }
}

impl Scope {
    /// The skill directory for one agent in this scope.
    fn skill_dir(&self, agent: Agent) -> PathBuf {
        match self {
            Scope::User(home) => agent.user_skill_dir(home),
            Scope::Project(root) => agent.project_skill_dir(root),
        }
    }
}

/// Map agents to their target skill directories, collapsing agents that
/// share a directory (e.g. codex/cursor/gemini at project level) into one
/// entry. BTreeMap keeps the output order deterministic.
fn dedupe_targets(scope: &Scope, agents: &[Agent]) -> BTreeMap<PathBuf, Vec<&'static str>> {
    let mut targets: BTreeMap<PathBuf, Vec<&'static str>> = BTreeMap::new();
    for agent in agents {
        let names = targets.entry(scope.skill_dir(*agent)).or_default();
        let name = agent.spec().name;
        if !names.contains(&name) {
            names.push(name);
        }
    }
    targets
}

/// Atomically write the embedded SKILL.md into `dir` (temp file in the same
/// directory, then rename — same approach as the binary self-update).
fn write_skill_md(dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create skill directory {}", dir.display()))?;
    let target = dir.join("SKILL.md");
    let tmp = tempfile::NamedTempFile::new_in(dir)
        .with_context(|| format!("failed to create temporary file in {}", dir.display()))?;
    tmp.as_file()
        .write_all(SKILL_MD.as_bytes())
        .with_context(|| format!("failed to write skill content in {}", dir.display()))?;
    tmp.persist(&target)
        .with_context(|| format!("failed to install skill at {}", target.display()))?;
    Ok(target)
}

/// Contract the home directory prefix to `~` for display.
fn display_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(rest) = path.strip_prefix(&home)
    {
        return format!("~/{}", rest.display());
    }
    path.display().to_string()
}

/// `nxv skill install` — write the embedded SKILL.md for the selected agents.
pub fn cmd_install(cli: &Cli, args: &SkillInstallArgs) -> Result<()> {
    let scope = resolve_scope(args.project, &args.dir)?;

    let agents: Vec<Agent> = if !args.agents.is_empty() {
        args.agents.clone()
    } else if args.detected {
        let home = dirs::home_dir().ok_or_else(|| {
            anyhow::anyhow!(
                "could not determine the home directory needed for --detected; name agents explicitly instead"
            )
        })?;
        let detected: Vec<Agent> = AGENTS
            .iter()
            .map(|spec| spec.agent)
            .filter(|agent| *agent != Agent::Agents && agent.is_detected(&home))
            .collect();
        if detected.is_empty() {
            anyhow::bail!(
                "no supported agents were detected; name agents explicitly, use `agents` for the generic Agent Skills directory, or use --all"
            );
        }
        detected
    } else if args.all {
        AGENTS.iter().map(|s| s.agent).collect()
    } else {
        anyhow::bail!("name at least one agent, or pass --detected or --all");
    };

    let targets = dedupe_targets(&scope, &agents);
    for (dir, names) in &targets {
        let target = write_skill_md(dir)?;
        if !cli.quiet {
            eprintln!(
                "Installed nxv skill: {} ({})",
                display_path(&target),
                names.join(", ")
            );
        }
    }

    Ok(())
}

/// `nxv skill uninstall` — remove installed copies of the skill. Only
/// `SKILL.md` is deleted; the `nxv/` directory is removed only when empty.
pub fn cmd_uninstall(cli: &Cli, args: &SkillUninstallArgs) -> Result<()> {
    let scope = resolve_scope(args.project, &args.dir)?;

    // With no agents named, sweep every known location in the scope.
    let agents: Vec<Agent> = if args.agents.is_empty() {
        AGENTS.iter().map(|s| s.agent).collect()
    } else {
        args.agents.clone()
    };

    let mut removed = 0usize;
    for (dir, _names) in dedupe_targets(&scope, &agents) {
        let skill_md = dir.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        std::fs::remove_file(&skill_md)
            .with_context(|| format!("failed to remove {}", skill_md.display()))?;
        removed += 1;
        if !cli.quiet {
            eprintln!("Removed nxv skill: {}", display_path(&skill_md));
        }
        // Best-effort cleanup of the now-empty nxv/ dir; leave it in place
        // if the user added supporting files next to SKILL.md.
        if std::fs::remove_dir(&dir).is_err() && !cli.quiet && dir.exists() {
            eprintln!("Left non-empty directory in place: {}", display_path(&dir));
        }
    }

    if removed == 0 && !cli.quiet {
        eprintln!("No installed nxv skill found in the selected scope.");
    }

    Ok(())
}

/// `nxv skill list` — table of agents, their skill paths, and install status.
pub fn cmd_list(_cli: &Cli, args: &SkillListArgs) -> Result<()> {
    use comfy_table::{ContentArrangement, Table, presets::ASCII_FULL, presets::UTF8_FULL};

    let home = dirs::home_dir();
    let project_root = std::env::current_dir().ok();

    let mut table = Table::new();
    table
        .load_preset(if args.ascii { ASCII_FULL } else { UTF8_FULL })
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            "Agent",
            "Name",
            "Detected",
            "User skill",
            "Project skill",
        ]);

    for spec in AGENTS {
        let detected = match &home {
            Some(home) => {
                if spec.agent.is_detected(home) {
                    "yes"
                } else {
                    "no"
                }
            }
            None => "-",
        };

        let user_cell = match &home {
            Some(home) => status_cell(&spec.agent.user_skill_dir(home)),
            None => "-".to_string(),
        };

        let project_cell = match &project_root {
            Some(root) => status_cell(&spec.agent.project_skill_dir(root)),
            None => "-".to_string(),
        };

        table.add_row(vec![
            label(spec.agent).to_string(),
            spec.name.to_string(),
            detected.to_string(),
            user_cell,
            project_cell,
        ]);
    }

    println!("{table}");
    Ok(())
}

/// Human-readable agent label for `skill list`.
fn label(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "Claude Code",
        Agent::Codex => "OpenAI Codex CLI",
        Agent::Pi => "Pi",
        Agent::Openclaw => "OpenClaw",
        Agent::Copilot => "GitHub Copilot CLI",
        Agent::Cursor => "Cursor",
        Agent::Gemini => "Gemini CLI",
        Agent::Amp => "Amp",
        Agent::Goose => "Goose",
        Agent::Agents => "Agent Skills (generic)",
    }
}

/// Path cell with an `[installed]` marker when SKILL.md exists there.
fn status_cell(dir: &Path) -> String {
    let path = display_path(dir);
    if dir.join("SKILL.md").exists() {
        format!("{path} [installed]")
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The user/project path mapping IS the contract with each agent — pin
    /// every entry.
    #[test]
    fn test_agent_path_mapping() {
        let home = Path::new("/home/u");
        let root = Path::new("/proj");
        let cases: &[(Agent, &str, &str)] = &[
            (Agent::Claude, ".claude/skills", ".claude/skills"),
            (Agent::Codex, ".codex/skills", ".agents/skills"),
            (Agent::Pi, ".pi/agent/skills", ".pi/skills"),
            (Agent::Openclaw, ".openclaw/skills", ".agents/skills"),
            (Agent::Copilot, ".copilot/skills", ".github/skills"),
            (Agent::Cursor, ".cursor/skills", ".agents/skills"),
            (Agent::Gemini, ".gemini/skills", ".agents/skills"),
            (Agent::Amp, ".config/amp/skills", ".agents/skills"),
            (Agent::Goose, ".config/goose/skills", ".agents/skills"),
            (Agent::Agents, ".agents/skills", ".agents/skills"),
        ];
        assert_eq!(cases.len(), AGENTS.len());
        for (agent, user, project) in cases {
            assert_eq!(
                agent.user_skill_dir(home),
                home.join(user).join("nxv"),
                "user path for {agent:?}"
            );
            assert_eq!(
                agent.project_skill_dir(root),
                root.join(project).join("nxv"),
                "project path for {agent:?}"
            );
        }
    }

    #[test]
    fn test_dedupe_shared_project_dirs() {
        let scope = Scope::Project(PathBuf::from("/proj"));
        let targets = dedupe_targets(&scope, &[Agent::Codex, Agent::Cursor, Agent::Gemini]);
        assert_eq!(targets.len(), 1);
        let (dir, names) = targets.iter().next().unwrap();
        assert_eq!(dir, &PathBuf::from("/proj/.agents/skills/nxv"));
        assert_eq!(names, &vec!["codex", "cursor", "gemini"]);
    }

    #[test]
    fn test_detection_only_sees_existing_config_dirs() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join(".codex")).unwrap();

        let detected: Vec<Agent> = AGENTS
            .iter()
            .map(|s| s.agent)
            .filter(|a| a.is_detected(home.path()))
            .collect();
        assert_eq!(detected, vec![Agent::Codex]);
    }

    /// Frontmatter must stay within the Agent Skills standard: single-line
    /// `key: value` pairs only (OpenClaw rejects folded scalars), required
    /// fields present, non-standard fields gone.
    #[test]
    fn test_skill_md_frontmatter_is_standard() {
        let rest = SKILL_MD
            .strip_prefix("---\n")
            .expect("SKILL.md must start with a frontmatter fence");
        let (frontmatter, body) = rest
            .split_once("\n---\n")
            .expect("SKILL.md frontmatter must have a closing fence");

        let mut keys = Vec::new();
        for line in frontmatter.lines() {
            let (key, value) = line.split_once(": ").unwrap_or_else(|| {
                panic!("frontmatter line is not single-line `key: value`: {line:?}")
            });
            assert!(
                key.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "frontmatter key is not lowercase-kebab: {key:?}"
            );
            assert!(
                !value.trim().is_empty(),
                "frontmatter value empty for {key:?}"
            );
            keys.push(key);

            if key == "description" {
                assert!(
                    (100..=1024).contains(&value.chars().count()),
                    "description must be 100..=1024 chars, got {}",
                    value.chars().count()
                );
            }
            if key == "name" {
                assert_eq!(value, "nxv", "skill name must match its directory name");
            }
        }

        assert!(keys.contains(&"name"), "frontmatter must have `name`");
        assert!(
            keys.contains(&"description"),
            "frontmatter must have `description`"
        );
        for forbidden in ["when_to_use", "argument-hint"] {
            assert!(
                !keys.contains(&forbidden),
                "non-standard frontmatter key present: {forbidden}"
            );
        }

        // The skill must document its own installer.
        assert!(body.contains("nxv skill install"));
    }

    #[test]
    fn test_write_skill_md_overwrites_and_is_verbatim() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("skills").join("nxv");

        let target = write_skill_md(&skill_dir).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), SKILL_MD);

        std::fs::write(&target, "stale garbage").unwrap();
        write_skill_md(&skill_dir).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), SKILL_MD);
    }
}
