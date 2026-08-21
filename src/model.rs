//! Core data model: Problem, Idea, and the local Ledger that tracks
//! what a user has bought / uninstalled / improved.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A problem is a searchable unit of intent, e.g. "agent 跨 session 记不住用户偏好".
/// Lives at `registry/problems/<slug>/problem.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// An idea is one candidate solution bound to a Problem, with a pointer to
/// readable source code. Multiple ideas under the same problem are peers —
/// there is no default ranking.
/// Lives at `registry/problems/<slug>/ideas/<idea-slug>.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idea {
    pub slug: String,
    pub problem_slug: String,
    pub title: String,
    /// Why this approach — the reasoning, not just what it does.
    pub rationale: String,
    pub author: String,
    pub source_repo: String,
    /// Path within source_repo to fetch (file or directory).
    pub source_path: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Idea {
    pub fn id(&self) -> String {
        format!("{}/{}", self.problem_slug, self.slug)
    }
}

/// A Problem bundled with all of its Ideas, as returned by search.
#[derive(Debug, Clone)]
pub struct ProblemWithIdeas {
    pub problem: Problem,
    pub ideas: Vec<Idea>,
}

/// Status of a locally bought idea.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeaStatus {
    /// Bought, not yet judged.
    Bought,
    /// Bought and modified — user recorded an improvement.
    Improved,
    /// Tried and rejected — the elimination signal.
    Rejected,
}

/// One entry in the local ledger: the full history of a single idea's
/// lifecycle in this user's repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub idea_id: String,
    pub status: IdeaStatus,
    pub bought_at: String,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub rejection_reason: Option<String>,
    #[serde(default)]
    pub improvement_note: Option<String>,
}

/// The full local ledger, persisted at ~/.nightwhale/ledger.json.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ledger {
    pub entries: BTreeMap<String, LedgerEntry>,
}
