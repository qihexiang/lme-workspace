use std::{collections::HashSet, sync::Arc};

use n_to_n::NtoN;

use crate::entity::{Layer, Molecule};

#[derive(Debug, Clone)]
pub struct Stack {
    current: Layer,
    classes: NtoN<String, usize>,
    cache: Molecule,
    base: Option<Arc<Stack>>,
}

impl Stack {
    pub fn new_empty(core_size: usize) -> Self {
        Self {
            current: Layer::Base(core_size),
            classes: NtoN::default(),
            cache: Molecule::default(),
            base: Option::default(),
        }
    }

    pub fn new(current: Layer, classes: NtoN<String, usize>, base: Arc<Stack>) -> Self {
        let cache = current.read(&base.cache);
        Self {
            current,
            classes,
            cache,
            base: Some(base),
        }
    }

    pub fn get_classes(&self, workspace_classes: &NtoN<String, usize>) -> NtoN<String, usize> {
        let mut classes = self.classes.clone();
        if let Some(base) = &self.base {
            let base: HashSet<_> = base.get_classes(workspace_classes).into();
            classes.extend(base);
        }
        let workspace: HashSet<_> = workspace_classes.clone().into();
        classes.extend(workspace);
        classes
    }

    pub fn read(&self) -> &Molecule {
        &self.cache
    }

    pub fn get_base(&self) -> Option<Arc<Self>> {
        self.base.clone()
    }

    pub fn get_layer(&self) -> Layer {
        self.current.clone()
    }
}
