use std::{collections::HashSet, sync::Arc};

use n_to_n::NtoN;

use crate::entity::{Layer, Molecule};

#[derive(Debug, Clone)]
pub struct Stack {
    top: Layer,
    cache: Molecule,
    base: Option<Arc<Stack>>,
}

impl Stack {
    pub fn new_empty(core_size: usize) -> Self {
        Self {
            top: Layer::Base(core_size),
            cache: Molecule::default(),
            base: Option::default(),
        }
    }

    pub fn new(top: Layer, base: Arc<Stack>) -> Self {
        let cache = top.read(&base.cache);
        Self {
            top,
            cache,
            base: Some(base),
        }
    }

    pub fn get_classes(&self, workspace_classes: &NtoN<String, usize>) -> NtoN<String, usize> {
        let mut classes = self.cache.classes.clone();
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

    pub fn get_top(&self) -> &Layer {
        &self.top
    }
}
