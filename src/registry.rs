//! Registry layer: reads Problems and Ideas out of the locally-synced
//! clone of nightwhale-dev/registry (GitHub repo used as the database).

use crate::model::{Idea, Problem, ProblemWithIdeas};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Registry {
    /// Path to the synced registry clone (~/.nightwhale/registry).
    root: PathBuf,
}

impl Registry {
    pub fn open(root: PathBuf) -> Self {
        Self { root }
    }

    fn problems_dir(&self) -> PathBuf {
        self.root.join("problems")
    }

    pub fn is_synced(&self) -> bool {
        self.problems_dir().exists()
    }

    /// The root used to resolve an idea's `source_path`. Source is vendored
    /// inside the registry repo itself, so this is just the registry root.
    pub fn source_root(&self) -> &Path {
        &self.root
    }

    /// Load every Problem with its Ideas. The registry is small (it's a git
    /// repo of yaml files), so eager loading is fine and keeps search simple.
    pub fn load_all(&self) -> Result<Vec<ProblemWithIdeas>> {
        let mut out = Vec::new();
        let problems_dir = self.problems_dir();
        if !problems_dir.exists() {
            return Ok(out);
        }

        for entry in fs::read_dir(&problems_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let pdir = entry.path();
            let problem = self.load_problem(&pdir)?;
            let ideas = self.load_ideas(&pdir, &problem.slug)?;
            out.push(ProblemWithIdeas { problem, ideas });
        }
        // Stable order by slug so output is deterministic — but note this is
        // NOT a ranking. Problems are peers; ordering is alphabetical only.
        out.sort_by(|a, b| a.problem.slug.cmp(&b.problem.slug));
        Ok(out)
    }

    fn load_problem(&self, pdir: &Path) -> Result<Problem> {
        let path = pdir.join("problem.yaml");
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let problem: Problem = serde_yaml::from_str(&raw)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(problem)
    }

    fn load_ideas(&self, pdir: &Path, problem_slug: &str) -> Result<Vec<Idea>> {
        let mut ideas = Vec::new();
        let idir = pdir.join("ideas");
        if !idir.exists() {
            return Ok(ideas);
        }
        for entry in fs::read_dir(&idir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let mut idea: Idea = serde_yaml::from_str(&raw)
                .with_context(|| format!("parsing {}", path.display()))?;
            // Trust the directory over the file for the problem linkage.
            idea.problem_slug = problem_slug.to_string();
            ideas.push(idea);
        }
        ideas.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(ideas)
    }

    /// Find a single idea by its "<problem-slug>/<idea-slug>" id.
    pub fn find_idea(&self, idea_id: &str) -> Result<Option<Idea>> {
        let (problem_slug, idea_slug) = match idea_id.split_once('/') {
            Some(parts) => parts,
            None => return Ok(None),
        };
        let path = self
            .problems_dir()
            .join(problem_slug)
            .join("ideas")
            .join(format!("{idea_slug}.yaml"));
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path)?;
        let mut idea: Idea = serde_yaml::from_str(&raw)
            .with_context(|| format!("parsing {}", path.display()))?;
        idea.problem_slug = problem_slug.to_string();
        Ok(Some(idea))
    }
}

/// Case-insensitive keyword + tag match against a problem and its ideas.
/// Matches if the query hits the problem title/description/tags OR any idea's
/// title/rationale/tags. Empty query matches everything (browse mode).
pub fn matches(pwi: &ProblemWithIdeas, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let terms: Vec<&str> = q.split_whitespace().collect();

    let mut haystack = String::new();
    haystack.push_str(&pwi.problem.title.to_lowercase());
    haystack.push(' ');
    haystack.push_str(&pwi.problem.description.to_lowercase());
    haystack.push(' ');
    haystack.push_str(&pwi.problem.tags.join(" ").to_lowercase());
    for idea in &pwi.ideas {
        haystack.push(' ');
        haystack.push_str(&idea.title.to_lowercase());
        haystack.push(' ');
        haystack.push_str(&idea.rationale.to_lowercase());
        haystack.push(' ');
        haystack.push_str(&idea.tags.join(" ").to_lowercase());
    }

    // All terms must appear (AND semantics) — narrows results usefully.
    terms.iter().all(|t| haystack.contains(t))
}
