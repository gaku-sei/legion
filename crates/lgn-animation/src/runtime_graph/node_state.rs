use crate::runtime_graph::node::Node;

pub struct StateNode {
    pub id: usize,
    pub child_node: Box<dyn Node>,
}

impl Node for StateNode {
    fn update(&mut self, _time: f32) {}
}
