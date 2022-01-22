use std::{borrow::Cow, fmt::Debug, hash::Hash};

use fixedbitset::FixedBitSet;
use lgn_tracing::warn;
use lgn_utils::{AHashExt, HashMap, HashSet};

pub enum DependencyGraphError<Labels> {
    GraphCycles(Vec<(usize, Labels)>),
}

pub trait GraphNode {
    type Label;
    fn name(&self) -> Cow<'static, str>;
    fn labels(&self) -> &[Self::Label];
    fn before(&self) -> &[Self::Label];
    fn after(&self) -> &[Self::Label];
}

/// Constructs a dependency graph of given nodes.
pub fn build_dependency_graph<Node>(
    nodes: &[Node],
) -> HashMap<usize, HashMap<usize, HashSet<Node::Label>>>
where
    Node: GraphNode,
    Node::Label: Debug + Clone + Eq + Hash,
{
    let mut labels = HashMap::<Node::Label, FixedBitSet>::default();
    for (label, index) in nodes.iter().enumerate().flat_map(|(index, container)| {
        container
            .labels()
            .iter()
            .cloned()
            .map(move |label| (label, index))
    }) {
        labels
            .entry(label)
            .or_insert_with(|| FixedBitSet::with_capacity(nodes.len()))
            .insert(index);
    }
    let mut graph = HashMap::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        let dependencies = graph.entry(index).or_insert_with(HashMap::default);
        for label in node.after() {
            if let Some(new_dependencies) = labels.get(label) {
                for dependency in new_dependencies.ones() {
                    dependencies
                        .entry(dependency)
                        .or_insert_with(HashSet::default)
                        .insert(label.clone());
                }
            } else {
                warn!(
                    // TODO: plumb this as proper output?
                    "{} wants to be after unknown label: {:?}",
                    nodes[index].name(),
                    label
                );
            }
        }
        for label in node.before() {
            if let Some(dependants) = labels.get(label) {
                for dependant in dependants.ones() {
                    graph
                        .entry(dependant)
                        .or_insert_with(HashMap::default)
                        .entry(index)
                        .or_insert_with(HashSet::default)
                        .insert(label.clone());
                }
            } else {
                warn!(
                    "{} wants to be before unknown label: {:?}",
                    nodes[index].name(),
                    label
                );
            }
        }
    }
    graph
}

/// Generates a topological order for the given graph.
#[allow(clippy::implicit_hasher)]
pub fn topological_order<Labels: Clone>(
    graph: &HashMap<usize, HashMap<usize, Labels>>,
) -> Result<Vec<usize>, DependencyGraphError<Labels>> {
    fn check_if_cycles_and_visit<L>(
        node: usize,
        graph: &HashMap<usize, HashMap<usize, L>>,
        sorted: &mut Vec<usize>,
        unvisited: &mut HashSet<usize>,
        current: &mut Vec<usize>,
    ) -> bool {
        if current.contains(&node) {
            return true;
        } else if !unvisited.remove(&node) {
            return false;
        }
        current.push(node);
        for dependency in graph.get(&node).unwrap().keys() {
            if check_if_cycles_and_visit(*dependency, graph, sorted, unvisited, current) {
                return true;
            }
        }
        sorted.push(node);
        current.pop();
        false
    }
    let mut sorted = Vec::with_capacity(graph.len());
    let mut current = Vec::with_capacity(graph.len());
    let mut unvisited = HashSet::with_capacity(graph.len());
    unvisited.extend(graph.keys().copied());
    while let Some(node) = unvisited.iter().next().copied() {
        if check_if_cycles_and_visit(node, graph, &mut sorted, &mut unvisited, &mut current) {
            let mut cycle = Vec::new();
            let last_window = [*current.last().unwrap(), current[0]];
            let mut windows = current
                .windows(2)
                .chain(std::iter::once(&last_window as &[usize]));
            while let Some(&[dependant, dependency]) = windows.next() {
                cycle.push((dependant, graph[&dependant][&dependency].clone()));
            }
            return Err(DependencyGraphError::GraphCycles(cycle));
        }
    }
    Ok(sorted)
}