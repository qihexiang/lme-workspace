use std::sync::Arc;

use n_to_n::NtoN;

pub struct Stack;

pub struct Layer;

pub struct Workspace {
    stacks: Vec<Arc<Stack>>,
    layers: Vec<Arc<Layer>>,
    ids: Vec<Option<String>>,
    classes: NtoN<usize, String>
}
