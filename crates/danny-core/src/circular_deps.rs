//! Circular dependency detection using Tarjan's algorithm.
//!
//! Detects strongly connected components (cycles) in the module dependency graph.
//! Time complexity: O(V + E) where V = modules, E = imports
//! Space complexity: O(V)

use crate::error::{Error, Result};
use crate::types::CircularDependency;
use std::collections::HashMap;
use std::path::PathBuf;

const MAX_MODULES: usize = 100_000;
const MAX_CYCLE_DEPTH: usize = 1_000;

/// Detects circular dependencies using Tarjan's SCC algorithm
pub struct CircularDependencyDetector {
    graph: HashMap<PathBuf, Vec<PathBuf>>,
    index_counter: usize,
    stack: Vec<PathBuf>,
    indices: HashMap<PathBuf, usize>,
    low_links: HashMap<PathBuf, usize>,
    on_stack: HashMap<PathBuf, bool>,
    sccs: Vec<Vec<PathBuf>>,
}

impl CircularDependencyDetector {
    /// Creates a new detector from module dependency data
    ///
    /// # Arguments
    /// * `dependencies` - Map of module path to its dependencies
    pub fn new(dependencies: HashMap<PathBuf, Vec<PathBuf>>) -> Self {
        Self {
            graph: dependencies,
            index_counter: 0,
            stack: Vec::new(),
            indices: HashMap::new(),
            low_links: HashMap::new(),
            on_stack: HashMap::new(),
            sccs: Vec::new(),
        }
    }

    /// Finds all strongly connected components (cycles)
    ///
    /// # Returns
    /// Vector of circular dependencies with cycle size >= 2
    ///
    /// # Errors
    /// - `Error::GraphTooLarge` if module count exceeds MAX_MODULES
    /// - `Error::CycleTooDeep` if cycle depth exceeds MAX_CYCLE_DEPTH
    pub fn find_cycles(&mut self) -> Result<Vec<CircularDependency>> {
        if self.graph.len() > MAX_MODULES {
            return Err(Error::GraphTooLarge {
                module_count: self.graph.len(),
                max_allowed: MAX_MODULES,
            });
        }

        let nodes: Vec<PathBuf> = self.graph.keys().cloned().collect();

        for node in nodes {
            if !self.indices.contains_key(&node) {
                self.strongconnect(node)?;
            }
        }

        // Filter to only actual cycles (size > 1)
        let cycles = self
            .sccs
            .iter()
            .filter(|scc| scc.len() > 1)
            .map(|cycle| CircularDependency {
                cycle: cycle.clone(),
                all_unreachable: false, // Set by caller
                total_size: 0,          // Set by caller
            })
            .collect();

        Ok(cycles)
    }

    /// Tarjan's algorithm - recursive strongconnect
    fn strongconnect(&mut self, v: PathBuf) -> Result<()> {
        if self.stack.len() > MAX_CYCLE_DEPTH {
            return Err(Error::CycleTooDeep {
                depth: self.stack.len(),
                max_allowed: MAX_CYCLE_DEPTH,
            });
        }

        self.indices.insert(v.clone(), self.index_counter);
        self.low_links.insert(v.clone(), self.index_counter);
        self.index_counter += 1;
        self.stack.push(v.clone());
        self.on_stack.insert(v.clone(), true);

        if let Some(neighbors) = self.graph.get(&v).cloned() {
            for w in neighbors {
                if !self.indices.contains_key(&w) {
                    self.strongconnect(w.clone())?;
                    let v_low = *self.low_links.get(&v).unwrap();
                    let w_low = *self.low_links.get(&w).unwrap();
                    self.low_links.insert(v.clone(), v_low.min(w_low));
                } else if *self.on_stack.get(&w).unwrap_or(&false) {
                    let v_low = *self.low_links.get(&v).unwrap();
                    let w_index = *self.indices.get(&w).unwrap();
                    self.low_links.insert(v.clone(), v_low.min(w_index));
                }
            }
        }

        // Found SCC root
        if self.low_links.get(&v) == self.indices.get(&v) {
            let mut scc = Vec::new();
            loop {
                let w = self.stack.pop().unwrap();
                self.on_stack.insert(w.clone(), false);
                scc.push(w.clone());
                if w == v {
                    break;
                }
            }
            self.sccs.push(scc);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_graph(edges: Vec<(&str, Vec<&str>)>) -> HashMap<PathBuf, Vec<PathBuf>> {
        edges
            .into_iter()
            .map(|(from, tos)| {
                (
                    PathBuf::from(from),
                    tos.into_iter().map(PathBuf::from).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn test_simple_cycle() {
        // A → B → C → A
        let graph = create_graph(vec![("A", vec!["B"]), ("B", vec!["C"]), ("C", vec!["A"])]);

        let mut detector = CircularDependencyDetector::new(graph);
        let cycles = detector.find_cycles().unwrap();

        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].cycle.len(), 3);
    }

    #[test]
    fn test_no_cycles() {
        // A → B → C (linear)
        let graph = create_graph(vec![("A", vec!["B"]), ("B", vec!["C"]), ("C", vec![])]);

        let mut detector = CircularDependencyDetector::new(graph);
        let cycles = detector.find_cycles().unwrap();

        assert_eq!(cycles.len(), 0);
    }

    #[test]
    fn test_multiple_cycles() {
        // A ↔ B and C ↔ D
        let graph = create_graph(vec![
            ("A", vec!["B"]),
            ("B", vec!["A"]),
            ("C", vec!["D"]),
            ("D", vec!["C"]),
        ]);

        let mut detector = CircularDependencyDetector::new(graph);
        let cycles = detector.find_cycles().unwrap();

        assert_eq!(cycles.len(), 2);
    }

    #[test]
    fn test_graph_too_large() {
        let mut graph = HashMap::new();
        for i in 0..150_000 {
            graph.insert(PathBuf::from(format!("mod{}", i)), vec![]);
        }

        let mut detector = CircularDependencyDetector::new(graph);
        let result = detector.find_cycles();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::GraphTooLarge { .. }));
    }
}
