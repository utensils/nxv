//! Self-update command — replaces the running `nxv` binary with the latest
//! release from GitHub, or prints guidance when managed by a package manager.
//!
//! The existing `nxv update` command continues to update the *index*; this
//! is a separate, dedicated path for updating the *binary* itself.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};

use crate::version;

const GITHUB_REPO: &str = "utensils/nxv";
const GITHUB_API_BASE: &str = "https://api.github.com";

// ── GitHub API types ────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(serde::Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

// ── Install method detection ────────────────────────────────────────────────

/// How the running `nxv` binary was installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMethod {
    /// Installed via Nix (path under `/nix/store/`).
    Nix,
    /// Installed via `cargo install` (path under `~/.cargo/bin/`).
    Cargo,
    /// Installed via Homebrew.
    Homebrew,
    /// A local/manual install — safe to replace in-place.
    Local,
}

impl InstallMethod {
    /// Short human-readable label for diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            InstallMethod::Nix => "Nix",
            InstallMethod::Cargo => "cargo install",
            InstallMethod::Homebrew => "Homebrew",
            InstallMethod::Local => "local install",
        }
    }

    /// Command (or instruction) the user should run to update a managed install.
    ///
    /// `force` and `version` influence the generated hint so that, e.g.,
    /// `nxv self-update --version v0.1.6` on a cargo install tells the user
    /// to run `cargo install --locked --version 0.1.6 nxv`, not the generic
    /// `cargo install --locked nxv` (which would silently install a different
    /// version). Returns `None` for `Local` — callers do the work themselves.
    pub fn update_hint(self, force: bool, version: Option<&str>) -> Option<String> {
        match self {
            InstallMethod::Nix => {
                if let Some(v) = version {
                    let tag = if v.starts_with('v') {
                        v.to_string()
                    } else {
                        format!("v{v}")
                    };
                    Some(format!(
                        "nix profile install --refresh github:utensils/nxv/{tag}  \
                         # or pin the flake input to {tag}"
                    ))
                } else {
                    Some("nix profile upgrade nxv  # or update your flake input".to_string())
                }
            }
            InstallMethod::Cargo => {
                let mut cmd = String::from("cargo install --locked");
                if force {
                    cmd.push_str(" --force");
                }
                if let Some(v) = version {
                    let bare = v.strip_prefix('v').unwrap_or(v);
                    cmd.push_str(" --version ");
                    cmd.push_str(bare);
                }
                cmd.push_str(" nxv");
                Some(cmd)
            }
            InstallMethod::Homebrew => {
                if version.is_some() {
                    // Homebrew tracks only the latest formula; pinning is not portable.
                    Some(String::from(
                        "# Homebrew tracks only the latest formula — use install.sh \
                         with NXV_VERSION=<tag> to install a specific release.",
                    ))
                } else if force {
                    Some(String::from("brew reinstall nxv"))
                } else {
                    Some(String::from("brew upgrade nxv"))
                }
            }
            InstallMethod::Local => None,
        }
    }
}

/// Classify the install method from the path of the running executable.
pub fn detect_install_method(exe_path: &Path) -> InstallMethod {
    let path_str = exe_path.to_string_lossy();

    if path_str.contains("/nix/store/") {
        return InstallMethod::Nix;
    }
    if path_str.contains("/Cellar/") || path_str.contains("/homebrew/") {
        return InstallMethod::Homebrew;
    }
    // `cargo install` drops binaries in $CARGO_HOME/bin (default ~/.cargo/bin).
    // Match `/.cargo/bin/` and the CARGO_HOME override, if set.
    if path_str.contains("/.cargo/bin/") {
        return InstallMethod::Cargo;
    }
    if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
        let bin = format!("{}/bin/", cargo_home.trim_end_matches('/'));
        if path_str.starts_with(&bin) {
            return InstallMethod::Cargo;
        }
    }
    InstallMethod::Local
}

// ── Version comparison ──────────────────────────────────────────────────────

/// Parse a version string like "0.1.7" or "v0.1.7" into `(major, minor, patch)`.
fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.strip_prefix('v').unwrap_or(v);
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

/// Returns true if `remote` is strictly newer than `current`.
fn is_newer(current: &str, remote: &str) -> bool {
    match (parse_version(current), parse_version(remote)) {
        (Some(c), Some(r)) => r > c,
        _ => false,
    }
}

// ── Platform detection ──────────────────────────────────────────────────────

/// The release asset name for the current platform.
///
/// Matches the names published by `.github/workflows/release.yml`:
///   - `nxv-x86_64-linux-musl`
///   - `nxv-aarch64-linux-musl`
///   - `nxv-x86_64-apple-darwin`
///   - `nxv-aarch64-apple-darwin`
fn detect_asset_name() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => Ok("nxv-x86_64-linux-musl".to_string()),
        ("linux", "aarch64") => Ok("nxv-aarch64-linux-musl".to_string()),
        ("macos", "x86_64") => Ok("nxv-x86_64-apple-darwin".to_string()),
        ("macos", "aarch64") => Ok("nxv-aarch64-apple-darwin".to_string()),
        _ => bail!("unsupported platform: {os}/{arch}"),
    }
}

// ── SHA-256 checksum verification ───────────────────────────────────────────

/// Parse a `SHA256SUMS.txt` file and verify the checksum for `asset_name` against `data`.
fn verify_checksum(sums_content: &str, asset_name: &str, data: &[u8]) -> Result<()> {
    let expected = sums_content
        .lines()
        .find_map(|line| {
            // Standard `sha256sum` output: "<hash>  <filename>" (two-space sep).
            let (hash, name) = line.split_once("  ")?;
            if name.trim() == asset_name {
                Some(hash.trim().to_string())
            } else {
                None
            }
        })
        .with_context(|| format!("asset {asset_name} not found in SHA256SUMS.txt"))?;

    let mut hasher = Sha256::new();
    hasher.update(data);
    let actual = format!("{:x}", hasher.finalize());

    if actual != expected {
        bail!(
            "SHA-256 checksum mismatch for {asset_name}\n  expected: {expected}\n  actual:   {actual}"
        );
    }
    Ok(())
}

// ── Binary self-replacement ─────────────────────────────────────────────────

/// Replace the binary at `exe_path` with `new_binary`.
///
/// On Unix this is a three-step dance: rename the current binary aside, install
/// the new one, and clean up the backup. If the install rename fails the
/// backup is restored. We can rename across the same directory because we
/// write the temp file next to the target.
///
/// `detect_asset_name` already bails on non-Unix platforms, so the Windows
/// stub below should never be reached at runtime — it exists purely to keep
/// `cargo build` green on targets we don't ship binaries for.
#[cfg(unix)]
fn replace_binary(new_binary: &[u8], exe_path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let exe_dir = exe_path
        .parent()
        .context("cannot determine binary directory")?;

    let pid = std::process::id();
    let tmp_path = exe_dir.join(format!(".nxv-update-{pid}"));
    // PID-suffixed backup so a crashed previous run or a concurrent update
    // attempt doesn't block the rename. `exe_path.with_extension("old")`
    // would always be `nxv.old`, which collides.
    let backup_path = exe_dir.join(format!(".nxv-backup-{pid}.old"));

    std::fs::write(&tmp_path, new_binary).context("failed to write new binary to temp file")?;
    std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
        .context("failed to set permissions on new binary")?;

    std::fs::rename(exe_path, &backup_path).with_context(|| {
        format!(
            "failed to move current binary to backup at {}",
            backup_path.display()
        )
    })?;

    if let Err(e) = std::fs::rename(&tmp_path, exe_path) {
        // Best-effort rollback. If it fails too, tell the user where the
        // backup lives so they can restore by hand.
        let rollback = std::fs::rename(&backup_path, exe_path);
        let _ = std::fs::remove_file(&tmp_path);
        if let Err(rb) = rollback {
            bail!(
                "failed to install new binary: {e}; rollback also failed: {rb}. \
                 Previous binary is preserved at {}",
                backup_path.display()
            );
        }
        bail!("failed to install new binary: {e}");
    }

    let _ = std::fs::remove_file(&backup_path);

    // macOS: strip the quarantine attribute so Gatekeeper doesn't prompt.
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("xattr")
            .args(["-d", "com.apple.quarantine"])
            .arg(exe_path)
            .output();
    }

    Ok(())
}

#[cfg(not(unix))]
fn replace_binary(_new_binary: &[u8], _exe_path: &Path) -> Result<()> {
    bail!("self-update is only supported on Unix platforms (Linux and macOS).")
}

// ── HTTP helpers ────────────────────────────────────────────────────────────

/// Build the HTTP client used for both GitHub API calls *and* binary downloads.
///
/// `connect_timeout_secs` bounds only the TCP/TLS handshake so we fail fast
/// on unreachable hosts. We intentionally do **not** set a total-request
/// timeout: the same client is reused for multi-MB binary downloads, and
/// the user can always Ctrl+C an unresponsive session.
fn build_client(connect_timeout_secs: u64) -> Result<reqwest::blocking::Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        "application/vnd.github+json".parse().expect("valid header"),
    );
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {token}")
                .parse()
                .context("invalid GITHUB_TOKEN")?,
        );
    }
    reqwest::blocking::Client::builder()
        .user_agent(format!("nxv/{}", version::PKG_VERSION))
        .default_headers(headers)
        .connect_timeout(Duration::from_secs(connect_timeout_secs))
        .build()
        .context("failed to build HTTP client")
}

fn fetch_latest_release(client: &reqwest::blocking::Client) -> Result<GitHubRelease> {
    let url = format!("{GITHUB_API_BASE}/repos/{GITHUB_REPO}/releases/latest");
    let resp = client
        .get(&url)
        .send()
        .context("failed to connect to GitHub API")?;
    handle_github_status(&resp)?;
    resp.json()
        .context("failed to parse GitHub release response")
}

fn fetch_release_by_tag(client: &reqwest::blocking::Client, tag: &str) -> Result<GitHubRelease> {
    // Accept both "v0.1.7" and "0.1.7". nxv tags are v-prefixed.
    let tag = if tag.starts_with('v') {
        tag.to_string()
    } else {
        format!("v{tag}")
    };
    let url = format!("{GITHUB_API_BASE}/repos/{GITHUB_REPO}/releases/tags/{tag}");
    let resp = client
        .get(&url)
        .send()
        .context("failed to connect to GitHub API")?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        bail!("release {tag} not found on GitHub");
    }
    handle_github_status(&resp)?;
    resp.json()
        .context("failed to parse GitHub release response")
}

fn handle_github_status(resp: &reqwest::blocking::Response) -> Result<()> {
    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        bail!(
            "GitHub API rate limit exceeded. Set GITHUB_TOKEN to authenticate:\n  \
             export GITHUB_TOKEN=$(gh auth token)"
        );
    }
    if !resp.status().is_success() {
        bail!("GitHub API returned {}", resp.status());
    }
    Ok(())
}

fn download_asset(
    client: &reqwest::blocking::Client,
    url: &str,
    size: u64,
    show_progress: bool,
) -> Result<Vec<u8>> {
    let mut resp = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .context("failed to download release asset")?;
    if !resp.status().is_success() {
        bail!("download failed with HTTP {}", resp.status());
    }

    let pb = if show_progress {
        let bar = ProgressBar::new(size);
        bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "  Downloading [{bar:30.cyan/blue}] \
                     {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                )
                .expect("valid template")
                .progress_chars("=>-"),
        );
        Some(bar)
    } else {
        None
    };

    let mut data = Vec::with_capacity(size as usize);
    let mut buf = [0u8; 64 * 1024];
    loop {
        use std::io::Read as _;
        let n = resp
            .read(&mut buf)
            .context("error reading download stream")?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        if let Some(bar) = &pb {
            bar.inc(n as u64);
        }
    }
    if let Some(bar) = pb {
        bar.finish_and_clear();
    }
    Ok(data)
}

// ── Main command ────────────────────────────────────────────────────────────

/// Options passed in from the CLI layer.
pub struct SelfUpdateOptions<'a> {
    pub check: bool,
    pub force: bool,
    pub version: Option<&'a str>,
    /// TCP/TLS connect timeout (seconds). Does **not** bound the download.
    pub connect_timeout_secs: u64,
    pub show_progress: bool,
    pub quiet: bool,
}

/// Check for a newer nxv release and, on a local install, replace the
/// running binary. On managed installs (Nix / cargo / Homebrew) this only
/// prints a package-manager-appropriate upgrade hint — the binary is never
/// touched. Returns `Ok(())` for all non-error outcomes (up-to-date,
/// managed-install hint, successful replacement).
pub fn run(opts: SelfUpdateOptions<'_>) -> Result<()> {
    let current = version::PKG_VERSION;
    if !opts.quiet {
        eprintln!("Checking for a newer nxv release...");
    }

    let client = build_client(opts.connect_timeout_secs)?;
    let release = match opts.version {
        Some(tag) => fetch_release_by_tag(&client, tag)?,
        None => fetch_latest_release(&client)?,
    };
    let remote_version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);

    // Version comparison (skipped under --force).
    if !opts.force {
        if remote_version == current {
            if !opts.quiet {
                eprintln!("Already up to date ({current}).");
            }
            return Ok(());
        }
        if opts.version.is_none() && !is_newer(current, remote_version) {
            if !opts.quiet {
                eprintln!(
                    "Current version ({current}) is newer than latest release ({remote_version})."
                );
            }
            return Ok(());
        }
    }

    let action = if is_newer(current, remote_version) {
        "Updating"
    } else if remote_version == current {
        "Reinstalling"
    } else {
        "Downgrading"
    };

    // `--check`: report availability and exit without touching disk.
    if opts.check {
        if !opts.quiet {
            if is_newer(current, remote_version) {
                eprintln!("New version available: {remote_version} (current: {current})");
            } else {
                eprintln!("Version {remote_version} is available (current: {current}).");
            }
        }
        return Ok(());
    }

    // From here on we will write to disk — validate install location.
    let exe_path = std::env::current_exe()
        .context("failed to locate current executable")?
        .canonicalize()
        .context("failed to canonicalize current executable path")?;
    let method = detect_install_method(&exe_path);

    if method != InstallMethod::Local {
        // Managed install: print the right hint and return — never touch the binary.
        eprintln!();
        eprintln!(
            "nxv was installed via {} ({}).",
            method.label(),
            exe_path.display()
        );
        if is_newer(current, remote_version) {
            eprintln!("A newer release is available: {remote_version} (current: {current}).");
        } else if remote_version == current {
            eprintln!("You are already on the latest release ({current}).");
        } else {
            eprintln!("Latest release is {remote_version}; you are on {current}.");
        }
        if let Some(hint) = method.update_hint(opts.force, opts.version) {
            eprintln!();
            eprintln!("To update, run:");
            eprintln!("  {hint}");
        }
        return Ok(());
    }

    // Local install — check we can actually write.
    if let Some(exe_dir) = exe_path.parent() {
        let test_path = exe_dir.join(format!(".nxv-update-test-{}", std::process::id()));
        match std::fs::write(&test_path, b"") {
            Ok(()) => {
                let _ = std::fs::remove_file(&test_path);
            }
            Err(_) => {
                bail!(
                    "no write permission to {}. Try running with sudo, \
                     or reinstall to a writable location.",
                    exe_dir.display()
                );
            }
        }
    }

    if !opts.quiet {
        eprintln!("{action}: {current} -> {remote_version}");
    }

    let asset_name = detect_asset_name()?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .with_context(|| {
            format!(
                "release {} has no asset matching {asset_name}",
                release.tag_name
            )
        })?;
    let sums_asset = release
        .assets
        .iter()
        .find(|a| a.name == "SHA256SUMS.txt")
        .context("release has no SHA256SUMS.txt file")?;

    let binary_data = download_asset(
        &client,
        &asset.browser_download_url,
        asset.size,
        opts.show_progress,
    )?;

    let sums_content = client
        .get(&sums_asset.browser_download_url)
        .send()
        .context("failed to download SHA256SUMS.txt")?
        // Fail loudly on a non-2xx — otherwise we'd feed an HTML error page
        // into checksum parsing and surface a misleading "asset not found" error.
        .error_for_status()
        .context("failed to download SHA256SUMS.txt (non-success status)")?
        .text()
        .context("failed to read SHA256SUMS.txt")?;

    verify_checksum(&sums_content, &asset.name, &binary_data)?;
    if !opts.quiet {
        eprintln!("Checksum verified (SHA-256).");
    }

    replace_binary(&binary_data, &exe_path)?;

    if !opts.quiet {
        eprintln!(
            "{action} complete: nxv {remote_version} ({}).",
            exe_path.display()
        );
    }
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── Version comparison ──────────────────────────────────────────────
    #[test]
    fn parse_version_valid() {
        assert_eq!(parse_version("0.1.7"), Some((0, 1, 7)));
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("10.20.30"), Some((10, 20, 30)));
    }

    #[test]
    fn parse_version_invalid() {
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("abc"), None);
        assert_eq!(parse_version("1.2.x"), None);
    }

    #[test]
    fn is_newer_basic() {
        assert!(is_newer("0.1.6", "0.1.7"));
        assert!(is_newer("0.1.7", "0.2.0"));
        assert!(!is_newer("0.1.7", "0.1.7"));
        assert!(!is_newer("1.0.0", "0.1.7"));
    }

    #[test]
    fn is_newer_with_v_prefix() {
        assert!(is_newer("0.1.6", "v0.1.7"));
        assert!(is_newer("v0.1.6", "0.1.7"));
        assert!(!is_newer("v0.2.0", "v0.1.9"));
    }

    #[test]
    fn is_newer_major_bump() {
        assert!(is_newer("0.9.9", "1.0.0"));
        assert!(!is_newer("1.0.0", "0.99.99"));
    }

    // ── Platform detection ──────────────────────────────────────────────
    #[test]
    fn detect_asset_name_current_platform() {
        let name = detect_asset_name().expect("current platform should be supported");
        assert!(name.starts_with("nxv-"));
    }

    // ── Install method detection ────────────────────────────────────────
    #[test]
    fn detect_nix_store() {
        let p = PathBuf::from("/nix/store/abc123-nxv/bin/nxv");
        assert_eq!(detect_install_method(&p), InstallMethod::Nix);
    }

    #[test]
    fn detect_homebrew() {
        let p = PathBuf::from("/opt/homebrew/Cellar/nxv/0.1.7/bin/nxv");
        assert_eq!(detect_install_method(&p), InstallMethod::Homebrew);
        let p2 = PathBuf::from("/usr/local/Cellar/nxv/0.1.7/bin/nxv");
        assert_eq!(detect_install_method(&p2), InstallMethod::Homebrew);
    }

    #[test]
    fn detect_cargo_bin() {
        let p = PathBuf::from("/Users/alice/.cargo/bin/nxv");
        assert_eq!(detect_install_method(&p), InstallMethod::Cargo);
        let p2 = PathBuf::from("/home/bob/.cargo/bin/nxv");
        assert_eq!(detect_install_method(&p2), InstallMethod::Cargo);
    }

    #[test]
    fn detect_local_bin() {
        let p = PathBuf::from("/home/user/.local/bin/nxv");
        assert_eq!(detect_install_method(&p), InstallMethod::Local);
        let p2 = PathBuf::from("/usr/local/bin/nxv");
        assert_eq!(detect_install_method(&p2), InstallMethod::Local);
    }

    #[test]
    fn install_method_update_hint_is_set_for_managed() {
        assert!(InstallMethod::Nix.update_hint(false, None).is_some());
        assert!(InstallMethod::Cargo.update_hint(false, None).is_some());
        assert!(InstallMethod::Homebrew.update_hint(false, None).is_some());
        assert!(InstallMethod::Local.update_hint(false, None).is_none());
    }

    #[test]
    fn cargo_hint_honors_version_and_force() {
        let hint = InstallMethod::Cargo
            .update_hint(true, Some("v0.1.6"))
            .unwrap();
        assert!(hint.contains("--force"), "hint missing --force: {hint}");
        assert!(
            hint.contains("--version 0.1.6"),
            "hint missing pinned version (stripped of v): {hint}"
        );
        assert!(hint.ends_with(" nxv"), "hint must target nxv: {hint}");
    }

    #[test]
    fn cargo_hint_plain() {
        let hint = InstallMethod::Cargo.update_hint(false, None).unwrap();
        assert_eq!(hint, "cargo install --locked nxv");
    }

    #[test]
    fn homebrew_hint_force_vs_version_vs_default() {
        let plain = InstallMethod::Homebrew.update_hint(false, None).unwrap();
        assert_eq!(plain, "brew upgrade nxv");
        let forced = InstallMethod::Homebrew.update_hint(true, None).unwrap();
        assert_eq!(forced, "brew reinstall nxv");
        // --version: Homebrew can't pin, so the hint becomes a guidance note.
        let pinned = InstallMethod::Homebrew
            .update_hint(false, Some("v0.1.6"))
            .unwrap();
        assert!(
            pinned.contains("install.sh"),
            "homebrew pin hint should redirect to install.sh: {pinned}"
        );
    }

    #[test]
    fn nix_hint_version_suggests_flake_ref() {
        let hint = InstallMethod::Nix
            .update_hint(false, Some("0.1.6"))
            .unwrap();
        assert!(
            hint.contains("github:utensils/nxv/v0.1.6"),
            "nix pin hint should include a flake ref: {hint}"
        );
    }

    // ── Checksum verification ───────────────────────────────────────────
    #[test]
    fn verify_checksum_match() {
        let data = b"hello nxv";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());
        let sums = format!("{hash}  nxv-x86_64-linux-musl\n");
        assert!(verify_checksum(&sums, "nxv-x86_64-linux-musl", data).is_ok());
    }

    #[test]
    fn verify_checksum_mismatch() {
        let sums = "0000000000000000000000000000000000000000000000000000000000000000  nxv-test\n";
        let err = verify_checksum(sums, "nxv-test", b"anything").unwrap_err();
        assert!(err.to_string().contains("checksum mismatch"));
    }

    #[test]
    fn verify_checksum_missing_asset() {
        let sums = "abcdef  other-file\n";
        let err = verify_checksum(sums, "nxv-test", b"data").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn verify_checksum_multi_line() {
        let a = b"file-a";
        let b = b"file-b";
        let ha = {
            let mut h = Sha256::new();
            h.update(a);
            format!("{:x}", h.finalize())
        };
        let hb = {
            let mut h = Sha256::new();
            h.update(b);
            format!("{:x}", h.finalize())
        };
        let sums = format!("{ha}  nxv-a\n{hb}  nxv-b\n");
        assert!(verify_checksum(&sums, "nxv-a", a).is_ok());
        assert!(verify_checksum(&sums, "nxv-b", b).is_ok());
    }

    // ── Binary replacement ──────────────────────────────────────────────
    #[cfg(unix)]
    #[test]
    fn replace_binary_roundtrip() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let exe_path = dir.path().join("nxv");
        std::fs::write(&exe_path, b"old").unwrap();
        std::fs::set_permissions(&exe_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        replace_binary(b"new-contents", &exe_path).unwrap();

        assert_eq!(std::fs::read(&exe_path).unwrap(), b"new-contents");
        let mode = std::fs::metadata(&exe_path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
        assert!(!exe_path.with_extension("old").exists());
    }

    #[cfg(unix)]
    #[test]
    fn replace_binary_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let exe_path = dir.path().join("nxv");
        std::fs::write(&exe_path, b"old").unwrap();

        replace_binary(b"fresh", &exe_path).unwrap();

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(".nxv-update-"))
            .collect();
        assert!(leftovers.is_empty(), "stray temp files: {leftovers:?}");
    }
}
