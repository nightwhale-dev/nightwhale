//! Local repo layer: manages ~/.nightwhale/ — the ledger, the synced
//! registry clone, and the bought ideas' source code.

use crate::model::{Idea, IdeaStatus, Ledger, LedgerEntry};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const REGISTRY_URL: &str = "https://github.com/nightwhale-dev/registry.git";

pub struct LocalRepo {
    root: PathBuf,
}

impl LocalRepo {
    /// Resolve ~/.nightwhale, creating the directory skeleton if missing.
    pub fn open() -> Result<Self> {
        let home = dirs::home_dir().context("could not resolve home directory")?;
        let root = home.join(".nightwhale");
        fs::create_dir_all(root.join("ideas"))?;
        Ok(Self { root })
    }

    pub fn registry_path(&self) -> PathBuf {
        self.root.join("registry")
    }

    pub fn ideas_path(&self) -> PathBuf {
        self.root.join("ideas")
    }

    pub fn idea_dir(&self, idea_id: &str) -> PathBuf {
        // idea_id is "<problem-slug>/<idea-slug>" — flatten to avoid nested
        // directory surprises while still being human-readable.
        self.ideas_path().join(idea_id.replace('/', "__"))
    }

    fn ledger_path(&self) -> PathBuf {
        self.root.join("ledger.json")
    }

    pub fn load_ledger(&self) -> Result<Ledger> {
        let path = self.ledger_path();
        if !path.exists() {
            return Ok(Ledger::default());
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("reading ledger at {}", path.display()))?;
        let ledger: Ledger = serde_json::from_str(&raw).context("parsing ledger.json")?;
        Ok(ledger)
    }

    pub fn save_ledger(&self, ledger: &Ledger) -> Result<()> {
        let raw = serde_json::to_string_pretty(ledger)?;
        fs::write(self.ledger_path(), raw)?;
        Ok(())
    }

    /// Clone the registry on first run, or `git pull` if it already exists.
    pub fn sync_registry(&self) -> Result<()> {
        let path = self.registry_path();
        if path.join(".git").exists() {
            run_git(&path, &["pull", "--ff-only"])?;
        } else {
            let parent = self.root.clone();
            run_git(&parent, &["clone", REGISTRY_URL, "registry"])?;
        }
        Ok(())
    }

    /// Copy an idea's source (file or directory) from the local registry
    /// clone's vendored path into ~/.nightwhale/ideas/<idea-id>/, plus a
    /// SOURCE.md provenance note. This assumes the registry entry points at
    /// a path *within the registry repo itself* (registry/problems/<slug>/vendor/...)
    /// — see registry layer for how external repos get vendored in.
    pub fn fetch_idea_source(&self, idea: &Idea, registry_source_root: &Path) -> Result<PathBuf> {
        let dest = self.idea_dir(&idea.id());
        if dest.exists() {
            bail!(
                "{} is already bought at {}. Uninstall it first if you want to re-fetch.",
                idea.id(),
                dest.display()
            );
        }
        fs::create_dir_all(&dest)?;

        let src = registry_source_root.join(&idea.source_path);
        if !src.exists() {
            bail!(
                "source path '{}' not found in registry (expected at {})",
                idea.source_path,
                src.display()
            );
        }
        copy_recursive(&src, &dest)?;

        let source_md = format!(
            "# Source\n\n\
             - Idea: {title}\n\
             - Author: {author}\n\
             - Origin repo: {repo}\n\
             - Origin path: {path}\n\
             - Rationale: {rationale}\n",
            title = idea.title,
            author = idea.author,
            repo = idea.source_repo,
            path = idea.source_path,
            rationale = idea.rationale,
        );
        fs::write(dest.join("SOURCE.md"), source_md)?;

        Ok(dest)
    }

    pub fn record_bought(&self, ledger: &mut Ledger, idea_id: &str) {
        ledger.entries.insert(
            idea_id.to_string(),
            LedgerEntry {
                idea_id: idea_id.to_string(),
                status: IdeaStatus::Bought,
                bought_at: now_iso(),
                updated_at: None,
                rejection_reason: None,
                improvement_note: None,
            },
        );
    }

    pub fn record_rejected(
        &self,
        ledger: &mut Ledger,
        idea_id: &str,
        reason: Option<String>,
    ) -> Result<()> {
        let entry = ledger
            .entries
            .get_mut(idea_id)
            .with_context(|| format!("{idea_id} was never bought — nothing to uninstall"))?;
        entry.status = IdeaStatus::Rejected;
        entry.updated_at = Some(now_iso());
        entry.rejection_reason = reason;
        Ok(())
    }

    pub fn record_improved(&self, ledger: &mut Ledger, idea_id: &str, note: String) -> Result<()> {
        let entry = ledger
            .entries
            .get_mut(idea_id)
            .with_context(|| format!("{idea_id} was never bought — nothing to improve"))?;
        entry.status = IdeaStatus::Improved;
        entry.updated_at = Some(now_iso());
        entry.improvement_note = Some(note);
        Ok(())
    }

    /// Remove the fetched source for an idea. Ledger entry is kept —
    /// rejection is a signal we want to remember, not just a deletion.
    pub fn remove_idea_source(&self, idea_id: &str) -> Result<()> {
        let dir = self.idea_dir(idea_id);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("failed to run `git {}`", args.join(" ")))?;
    if !status.success() {
        bail!("`git {}` exited with {}", args.join(" "), status);
    }
    Ok(())
}

fn copy_recursive(src: &Path, dest: &Path) -> Result<()> {
    if src.is_dir() {
        for entry in walkdir::WalkDir::new(src).min_depth(1) {
            let entry = entry?;
            let rel = entry.path().strip_prefix(src)?;
            let target = dest.join(rel);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&target)?;
            } else {
                fs::create_dir_all(target.parent().unwrap())?;
                fs::copy(entry.path(), &target)?;
            }
        }
    } else {
        let file_name = src
            .file_name()
            .context("source path has no file name")?;
        fs::copy(src, dest.join(file_name))?;
    }
    Ok(())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}
