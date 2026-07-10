use std::collections::HashSet;

use serde::Serialize;

use crate::domain::MemoryItem;
use crate::error::SynapseError;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub track: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Builds a renderable graph from every item's `related_ids`. Since links are
/// stored symmetrically on both sides, each pair is de-duplicated into a
/// single edge.
pub fn build_graph(items: &[MemoryItem]) -> KnowledgeGraph {
    let nodes = items
        .iter()
        .map(|item| GraphNode {
            id: item.id.clone(),
            label: item.prompt.clone(),
            track: item.training_track.clone(),
        })
        .collect();

    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    for item in items {
        for related_id in &item.related_ids {
            let mut pair = [item.id.clone(), related_id.clone()];
            pair.sort();
            let [source, target] = pair;
            if seen.insert((source.clone(), target.clone())) {
                edges.push(GraphEdge { source, target });
            }
        }
    }

    KnowledgeGraph { nodes, edges }
}

/// Symmetrically links two memories (adds each id to the other's
/// `related_ids`). Idempotent: linking an already-linked pair is a no-op.
pub fn link(items: &mut [MemoryItem], id_a: &str, id_b: &str) -> Result<(), SynapseError> {
    if id_a == id_b {
        return Err(SynapseError::InvalidOperation(
            "cannot link a memory to itself".to_string(),
        ));
    }
    if !items.iter().any(|i| i.id == id_a) {
        return Err(SynapseError::NotFound(id_a.to_string()));
    }
    if !items.iter().any(|i| i.id == id_b) {
        return Err(SynapseError::NotFound(id_b.to_string()));
    }

    for item in items.iter_mut() {
        if item.id == id_a && !item.related_ids.iter().any(|r| r == id_b) {
            item.related_ids.push(id_b.to_string());
        } else if item.id == id_b && !item.related_ids.iter().any(|r| r == id_a) {
            item.related_ids.push(id_a.to_string());
        }
    }
    Ok(())
}

/// Symmetrically removes a link between two memories. No-op if not linked.
pub fn unlink(items: &mut [MemoryItem], id_a: &str, id_b: &str) {
    for item in items.iter_mut() {
        if item.id == id_a {
            item.related_ids.retain(|r| r != id_b);
        } else if item.id == id_b {
            item.related_ids.retain(|r| r != id_a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CardContent;

    fn item(track: &str, prompt: &str) -> MemoryItem {
        MemoryItem::new(track, prompt, CardContent::basic("..."))
    }

    #[test]
    fn link_is_symmetric_and_deduplicated_in_the_graph() {
        let mut items = vec![item("Rust", "What is a trait?"), item("Rust", "What is a trait object?")];
        let (id_a, id_b) = (items[0].id.clone(), items[1].id.clone());

        link(&mut items, &id_a, &id_b).unwrap();
        assert!(items[0].related_ids.contains(&id_b));
        assert!(items[1].related_ids.contains(&id_a));

        // linking again is a no-op, not a duplicate
        link(&mut items, &id_a, &id_b).unwrap();
        assert_eq!(items[0].related_ids.len(), 1);

        let graph = build_graph(&items);
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn linking_to_self_or_missing_id_errors() {
        let mut items = vec![item("Rust", "What is a trait?")];
        let id = items[0].id.clone();

        assert!(matches!(
            link(&mut items, &id, &id),
            Err(SynapseError::InvalidOperation(_))
        ));
        assert!(matches!(
            link(&mut items, &id, "missing"),
            Err(SynapseError::NotFound(_))
        ));
    }

    #[test]
    fn unlink_removes_both_sides() {
        let mut items = vec![item("Rust", "A"), item("Rust", "B")];
        let (id_a, id_b) = (items[0].id.clone(), items[1].id.clone());
        link(&mut items, &id_a, &id_b).unwrap();

        unlink(&mut items, &id_a, &id_b);
        assert!(items[0].related_ids.is_empty());
        assert!(items[1].related_ids.is_empty());
    }
}
