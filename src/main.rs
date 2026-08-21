//! NightWhale — an idea exchange for the DeepSeek Harness (dsh) ecosystem.
//!
//! Core stance (from the PRD): there is no single "best" solution to a
//! problem. Ideas under a problem are peers. NightWhale makes multiple
//! solutions *visible* and lets the developer choose — it never collapses
//! them into one recommendation.

mod local_repo;
mod model;
mod registry;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use local_repo::LocalRepo;
use model::IdeaStatus;
use registry::Registry;

#[derive(Parser)]
#[command(
    name = "nightwhale",
    version,
    about = "夜鲲 — an idea exchange for the dsh ecosystem. Search, buy, decompose, personalize."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Sync the public idea registry (clone or pull).
    Sync,
    /// Search problems and their candidate ideas by keyword/tag.
    Search {
        /// Query terms. Omit to browse everything.
        #[arg(default_value = "")]
        query: String,
    },
    /// Buy an idea: fetch its source into your local repo. ID is "problem/idea".
    Buy {
        idea_id: String,
    },
    /// Uninstall a bought idea and record it as tried-and-rejected.
    Uninstall {
        idea_id: String,
        /// Optional reason — the elimination signal is more useful with a why.
        #[arg(short, long)]
        reason: Option<String>,
    },
    /// Record that you've improved a bought idea (keeps the source in place).
    Improve {
        idea_id: String,
        #[arg(short, long)]
        note: String,
    },
    /// List everything in your local ledger.
    List,
    /// Generate a PR-ready yaml template to propose a new idea to the registry.
    Propose {
        /// The problem slug this idea answers (existing or new).
        #[arg(short, long)]
        problem: String,
        /// Short slug for the new idea.
        #[arg(short, long)]
        idea: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Sync => cmd_sync(),
        Commands::Search { query } => cmd_search(&query),
        Commands::Buy { idea_id } => cmd_buy(&idea_id),
        Commands::Uninstall { idea_id, reason } => cmd_uninstall(&idea_id, reason),
        Commands::Improve { idea_id, note } => cmd_improve(&idea_id, note),
        Commands::List => cmd_list(),
        Commands::Propose { problem, idea } => cmd_propose(&problem, &idea),
    }
}

fn open_registry(repo: &LocalRepo) -> Result<Registry> {
    let reg = Registry::open(repo.registry_path());
    if !reg.is_synced() {
        bail!("registry not synced yet — run `nightwhale sync` first");
    }
    Ok(reg)
}

fn cmd_sync() -> Result<()> {
    let repo = LocalRepo::open()?;
    println!("Syncing registry from {} …", local_repo::REGISTRY_URL);
    repo.sync_registry()?;
    let reg = Registry::open(repo.registry_path());
    let all = reg.load_all()?;
    let idea_count: usize = all.iter().map(|p| p.ideas.len()).sum();
    println!(
        "✓ Synced. {} problems, {} ideas available.",
        all.len(),
        idea_count
    );
    Ok(())
}

fn cmd_search(query: &str) -> Result<()> {
    let repo = LocalRepo::open()?;
    let reg = open_registry(&repo)?;
    let all = reg.load_all()?;

    let hits: Vec<_> = all
        .iter()
        .filter(|pwi| registry::matches(pwi, query))
        .collect();

    if hits.is_empty() {
        println!("No problems matched \"{query}\".");
        return Ok(());
    }

    let ledger = repo.load_ledger()?;

    for pwi in hits {
        println!("\n\x1b[1m{}\x1b[0m  ({})", pwi.problem.title, pwi.problem.slug);
        println!("  {}", pwi.problem.description);
        if !pwi.problem.tags.is_empty() {
            println!("  tags: {}", pwi.problem.tags.join(", "));
        }
        println!("  {} candidate idea(s) — you choose, no ranking:", pwi.ideas.len());
        for idea in &pwi.ideas {
            let marker = match ledger.entries.get(&idea.id()).map(|e| e.status) {
                Some(IdeaStatus::Bought) => " [bought]",
                Some(IdeaStatus::Improved) => " [improved]",
                Some(IdeaStatus::Rejected) => " [rejected]",
                None => "",
            };
            println!("    • \x1b[36m{}\x1b[0m — {}{}", idea.id(), idea.title, marker);
            println!("        {}", idea.rationale);
            println!("        source: {} :: {}", idea.source_repo, idea.source_path);
        }
    }
    println!("\nBuy one with:  nightwhale buy <problem/idea>");
    Ok(())
}

fn cmd_buy(idea_id: &str) -> Result<()> {
    let repo = LocalRepo::open()?;
    let reg = open_registry(&repo)?;

    let idea = match reg.find_idea(idea_id)? {
        Some(i) => i,
        None => bail!("no idea with id '{idea_id}' — check `nightwhale search`"),
    };

    let dest = repo.fetch_idea_source(&idea, reg.source_root())?;
    let mut ledger = repo.load_ledger()?;
    repo.record_bought(&mut ledger, idea_id);
    repo.save_ledger(&ledger)?;

    println!("✓ Bought \x1b[36m{}\x1b[0m", idea_id);
    println!("  source → {}", dest.display());
    println!("  Read the code, use it, then tell us what happened:");
    println!("    keeps working?  nightwhale improve {idea_id} --note \"...\"");
    println!("    didn't help?    nightwhale uninstall {idea_id} --reason \"...\"");
    Ok(())
}

fn cmd_uninstall(idea_id: &str, reason: Option<String>) -> Result<()> {
    let repo = LocalRepo::open()?;
    let mut ledger = repo.load_ledger()?;
    repo.record_rejected(&mut ledger, idea_id, reason)?;
    repo.remove_idea_source(idea_id)?;
    repo.save_ledger(&ledger)?;
    println!("✓ Uninstalled {idea_id}. Recorded as tried-and-rejected.");
    println!("  (The rejection signal stays in your ledger — that's the point.)");
    Ok(())
}

fn cmd_improve(idea_id: &str, note: String) -> Result<()> {
    let repo = LocalRepo::open()?;
    let mut ledger = repo.load_ledger()?;
    repo.record_improved(&mut ledger, idea_id, note)?;
    repo.save_ledger(&ledger)?;
    println!("✓ Marked {idea_id} as improved.");
    println!("  Your modified copy lives at {}", repo.idea_dir(idea_id).display());
    println!("  When it's good enough to share: nightwhale propose ...");
    Ok(())
}

fn cmd_list() -> Result<()> {
    let repo = LocalRepo::open()?;
    let ledger = repo.load_ledger()?;
    if ledger.entries.is_empty() {
        println!("Your ledger is empty. Find something: nightwhale search");
        return Ok(());
    }
    println!("Your idea ledger ({} entries):\n", ledger.entries.len());
    for entry in ledger.entries.values() {
        let status = match entry.status {
            IdeaStatus::Bought => "\x1b[33mbought\x1b[0m",
            IdeaStatus::Improved => "\x1b[32mimproved\x1b[0m",
            IdeaStatus::Rejected => "\x1b[31mrejected\x1b[0m",
        };
        println!("  {} — {}", entry.idea_id, status);
        if let Some(r) = &entry.rejection_reason {
            println!("      reason: {r}");
        }
        if let Some(n) = &entry.improvement_note {
            println!("      note: {n}");
        }
    }
    Ok(())
}

fn cmd_propose(problem: &str, idea: &str) -> Result<()> {
    let template = format!(
        "# Copy this into the registry as:\n\
         #   problems/{problem}/problem.yaml   (if the problem is new)\n\
         #   problems/{problem}/ideas/{idea}.yaml\n\
         # then open a PR to https://github.com/nightwhale-dev/registry\n\
         \n\
         # --- problem.yaml (skip if the problem already exists) ---\n\
         slug: {problem}\n\
         title: \"<one-line problem statement>\"\n\
         description: \"<what's the actual pain, in whose words>\"\n\
         tags: []\n\
         \n\
         # --- ideas/{idea}.yaml ---\n\
         slug: {idea}\n\
         problem_slug: {problem}\n\
         title: \"<your approach in a phrase>\"\n\
         rationale: \"<WHY this way — the insight, not just the what>\"\n\
         author: \"<your handle>\"\n\
         source_repo: \"<url to the readable source>\"\n\
         source_path: \"problems/{problem}/vendor/{idea}\"  # vendor the code into the registry\n\
         tags: []\n"
    );
    println!("{template}");
    Ok(())
}
