//! Deterministic Dependency Resolution for Equipment Packages and Templates.
//!
//! Provides topological sorting, cycle detection, missing dependency detection,
//! version compatibility matching, and deterministic resolution ordering.

use nebudcs_types::SchemaVersion;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::error::{ScriptingError, ScriptingResult};
use crate::library::package::PackageDependency;

/// Simple semantic version constraint matcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionConstraint {
    /// Matches any version ("*")
    Any,
    /// Matches exact version ("1.2.3")
    Exact(SchemaVersion),
    /// Matches compatible minor/patch ("^1.2.0" => major 1, >= 1.2.0)
    Caret(SchemaVersion),
    /// Matches greater than or equal (">=1.0.0")
    GreaterOrEqual(SchemaVersion),
}

impl VersionConstraint {
    /// Parses a version constraint string.
    pub fn parse(s: &str) -> ScriptingResult<Self> {
        let trimmed = s.trim();
        if trimmed == "*" || trimmed.is_empty() {
            return Ok(Self::Any);
        }

        if let Some(rest) = trimmed.strip_prefix('^') {
            let ver = rest
                .trim()
                .parse::<SchemaVersion>()
                .map_err(|e| ScriptingError::Dependency(format!("Invalid caret version: {e}")))?;
            return Ok(Self::Caret(ver));
        }

        if let Some(rest) = trimmed.strip_prefix(">=") {
            let ver = rest
                .trim()
                .parse::<SchemaVersion>()
                .map_err(|e| ScriptingError::Dependency(format!("Invalid >= version: {e}")))?;
            return Ok(Self::GreaterOrEqual(ver));
        }

        // Exact version
        let ver = trimmed
            .parse::<SchemaVersion>()
            .map_err(|e| ScriptingError::Dependency(format!("Invalid exact version: {e}")))?;
        Ok(Self::Exact(ver))
    }

    /// Checks if a concrete SchemaVersion satisfies this constraint.
    pub fn matches(&self, version: &SchemaVersion) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(v) => version == v,
            Self::Caret(v) => {
                if version.major != v.major {
                    false
                } else if version.minor > v.minor {
                    true
                } else if version.minor == v.minor {
                    version.patch >= v.patch
                } else {
                    false
                }
            }
            Self::GreaterOrEqual(v) => version >= v,
        }
    }
}

/// Dependency graph node for topological ordering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyNode {
    pub id: String,
    pub version: SchemaVersion,
}

/// Resolves dependencies among templates and packages.
#[derive(Default)]
pub struct DependencyResolver {
    /// Directed edges: dependent -> list of (dependency_id, constraint)
    dependencies: BTreeMap<String, Vec<PackageDependency>>,
}

impl DependencyResolver {
    /// Creates a new dependency resolver.
    pub fn new() -> Self {
        Self {
            dependencies: BTreeMap::new(),
        }
    }

    /// Registers dependencies for a package or template.
    pub fn add_dependencies(&mut self, id: impl Into<String>, deps: Vec<PackageDependency>) {
        self.dependencies.insert(id.into(), deps);
    }

    /// Validates and resolves all registered nodes in deterministic topological order.
    ///
    /// Checks:
    /// - Missing dependencies against `available_versions` map.
    /// - Version constraint mismatches.
    /// - Version conflicts (different dependencies requiring mutually incompatible versions).
    /// - Cyclic dependencies.
    ///
    /// Returns deterministic resolution order (leaves first, dependents last).
    pub fn resolve(
        &self,
        available_versions: &HashMap<String, Vec<SchemaVersion>>,
    ) -> ScriptingResult<Vec<String>> {
        // 1. Validate all dependencies exist and satisfy constraints
        let mut selected_versions: HashMap<String, SchemaVersion> = HashMap::new();

        for (node_id, deps) in &self.dependencies {
            for dep in deps {
                let versions = available_versions.get(&dep.id).ok_or_else(|| {
                    ScriptingError::Dependency(format!(
                        "Dependency '{}' required by '{}' was not found in library",
                        dep.id, node_id
                    ))
                })?;

                let constraint = VersionConstraint::parse(&dep.version_req)?;
                let matching_version = versions
                    .iter()
                    .filter(|v| constraint.matches(v))
                    .max()
                    .ok_or_else(|| {
                        ScriptingError::Dependency(format!(
                            "No available version of '{}' satisfies constraint '{}' for '{}' (available: {:?})",
                            dep.id, dep.version_req, node_id, versions
                        ))
                    })?;

                if let Some(existing) = selected_versions.get(&dep.id) {
                    if !constraint.matches(existing) {
                        return Err(ScriptingError::Dependency(format!(
                            "Version conflict for '{}': already resolved to '{}', but '{}' requires '{}'",
                            dep.id, existing, node_id, dep.version_req
                        )));
                    }
                } else {
                    selected_versions.insert(dep.id.clone(), *matching_version);
                }
            }
        }

        // 2. Build graph for topological sorting (Kahn's algorithm with BTree for determinism)
        let mut all_nodes: BTreeSet<String> = BTreeSet::new();
        for (k, deps) in &self.dependencies {
            all_nodes.insert(k.clone());
            for dep in deps {
                all_nodes.insert(dep.id.clone());
            }
        }

        // adjacency: from dependency -> list of dependents (dep -> dependent)
        let mut adj: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();

        for node in &all_nodes {
            adj.entry(node.clone()).or_default();
            in_degree.entry(node.clone()).or_insert(0);
        }

        for (dependent, deps) in &self.dependencies {
            for dep in deps {
                if adj.get_mut(&dep.id).unwrap().insert(dependent.clone()) {
                    *in_degree.get_mut(dependent).unwrap() += 1;
                }
            }
        }

        // Queue of nodes with 0 dependencies (in_degree == 0)
        let mut ready: BTreeSet<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut order = Vec::new();

        while let Some(current) = ready.pop_first() {
            order.push(current.clone());

            if let Some(dependents) = adj.get(&current) {
                for dep in dependents {
                    let deg = in_degree.get_mut(dep).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        ready.insert(dep.clone());
                    }
                }
            }
        }

        // Cycle check
        if order.len() < all_nodes.len() {
            // Find a cycle path for actionable diagnostics
            let unvisited: Vec<String> = all_nodes
                .iter()
                .filter(|n| !order.contains(n))
                .cloned()
                .collect();
            return Err(ScriptingError::Dependency(format!(
                "Dependency cycle detected among: {:?}",
                unvisited
            )));
        }

        Ok(order)
    }
}
