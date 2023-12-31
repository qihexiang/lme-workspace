use std::sync::Arc;

pub use n_to_n::NtoN;

pub mod config;
pub mod entity;
pub mod stack;

use entity::{Layer, Molecule};
pub use nalgebra;
use rayon::prelude::*;
use serde::Serialize;
use stack::Stack;

#[derive(Debug, Clone)]
pub struct Workspace {
    core_size: usize,
    stacks: Vec<Arc<Stack>>,
    ids: Vec<Option<String>>,
    classes: NtoN<String, usize>,
}

impl Workspace {
    pub fn new(core_size: usize) -> Self {
        Self {
            core_size,
            stacks: vec![Arc::new(Stack::new_empty(core_size))],
            ids: vec![],
            classes: NtoN::new(),
        }
    }

    pub fn exist_stack(&self, stack_idx: usize) -> bool {
        self.stacks.len() > stack_idx
    }

    pub fn create_stacks_from(
        &mut self,
        stack_idx: usize,
        copies: usize,
    ) -> Result<usize, WorkspaceError> {
        if let Some(stack) = self.stacks.get(stack_idx).cloned() {
            let start = self.stacks.len();
            for _ in 0..copies {
                self.stacks.push(stack.clone());
            }
            Ok(start)
        } else {
            Err(WorkspaceError::StackNotFound)
        }
    }

    pub fn detach_base_from(
        &mut self,
        stack_idx: usize,
        copies: usize,
    ) -> Result<usize, WorkspaceError> {
        if let Some(stack) = self.stacks.get(stack_idx).cloned() {
            let start = self.stacks.len();
            let base = stack
                .get_base()
                .unwrap_or(Arc::new(Stack::new_empty(self.core_size)));
            for _ in 0..copies {
                self.stacks.push(base.clone());
            }
            Ok(start)
        } else {
            Err(WorkspaceError::StackNotFound)
        }
    }

    pub fn get_classes(&self) -> &NtoN<String, usize> {
        &self.classes
    }

    pub fn read_stack(&self, stack_idx: usize) -> Result<Arc<Stack>, WorkspaceError> {
        if let Some(stack) = self.stacks.get(stack_idx) {
            Ok(stack.clone())
        } else {
            Err(WorkspaceError::StackNotFound)
        }
    }

    pub fn write_to_stacks(
        &mut self,
        stack_idxs: &Vec<usize>,
        patch: &Molecule,
    ) -> Result<(), WorkspaceError> {
        let currents = stack_idxs
            .par_iter()
            .copied()
            .map(|stack_idx| self.stacks.get(stack_idx))
            .collect::<Vec<_>>();
        if currents.par_iter().all(|item| item.is_some()) {
            let writtens = currents
                .into_par_iter()
                .map(|item| item.expect("Checked no None here"))
                .map(|stack| {
                    if let Layer::Fill(molecule) = stack.get_top() {
                        let molecule = patch.overlay_to(molecule);
                        let layer = Layer::Fill(molecule);
                        Arc::new(Stack::new(layer, stack.clone()))
                    } else {
                        Arc::new(Stack::new(Layer::Fill(patch.clone()), stack.clone()))
                    }
                })
                .collect::<Vec<_>>();
            let patch = stack_idxs
                .par_iter()
                .copied()
                .zip(writtens.into_par_iter())
                .collect::<Vec<_>>();
            for (stack_idx, updated) in patch {
                self.stacks[stack_idx] = updated
            }
            Ok(())
        } else {
            Err(WorkspaceError::StackNotFound)
        }
    }

    pub fn write_to_stack(
        &mut self,
        stack_idx: usize,
        patch: &Molecule,
    ) -> Result<(), WorkspaceError> {
        if let Some(stack) = self.stacks.get_mut(stack_idx) {
            if let Layer::Fill(molecule) = stack.get_top() {
                let molecule = patch.overlay_to(molecule);
                let layer = Layer::Fill(molecule);
                *stack = Arc::new(Stack::new(layer, stack.clone()))
            } else {
                *stack = Arc::new(Stack::new(Layer::Fill(patch.clone()), stack.clone()));
            }
            Ok(())
        } else {
            Err(WorkspaceError::StackNotFound)
        }
    }

    pub fn overlay_to_stacks(
        &mut self,
        stack_idxs: &Vec<usize>,
        layer: &Layer,
    ) -> Result<(), WorkspaceError> {
        let currents = stack_idxs
            .par_iter()
            .copied()
            .map(|stack_idx| self.stacks.get(stack_idx))
            .collect::<Vec<_>>();
        if currents.par_iter().all(|item| item.is_some()) {
            let writtens = currents
                .into_par_iter()
                .map(|item| item.expect("Checked not None here"))
                .map(|stack| Arc::new(Stack::new(layer.clone(), stack.clone())))
                .collect::<Vec<_>>();
            let patch = stack_idxs
                .par_iter()
                .copied()
                .zip(writtens.into_par_iter())
                .collect::<Vec<_>>();
            for (stack_idx, updated) in patch {
                self.stacks[stack_idx] = updated;
            }
            Ok(())
        } else {
            Err(WorkspaceError::StackNotFound)
        }
    }

    pub fn overlay_to_stack(
        &mut self,
        stack_idx: usize,
        layer: &Layer,
    ) -> Result<&Molecule, WorkspaceError> {
        if let Some(stack) = self.stacks.get_mut(stack_idx) {
            *stack = Arc::new(Stack::new(layer.clone(), stack.clone()));
            Ok(stack.read())
        } else {
            Err(WorkspaceError::StackNotFound)
        }
    }

    pub fn remove_stack(&mut self, stack_idx: usize) -> Result<Arc<Stack>, WorkspaceError> {
        if let Ok(stack) = self.read_stack(stack_idx) {
            self.stacks.remove(stack_idx);
            Ok(stack)
        } else {
            Err(WorkspaceError::StackNotFound)
        }
    }

    pub fn set_name(&mut self, atom_idx: usize, id_name: String) -> Result<(), WorkspaceError> {
        if atom_idx < self.core_size {
            self.ids[atom_idx] = Some(id_name);
            Ok(())
        } else {
            Err(WorkspaceError::IndexOutOfCoreSize)
        }
    }

    pub fn get_name(&self, atom_idx: usize) -> Result<Option<String>, WorkspaceError> {
        if atom_idx < self.core_size {
            Ok(self.ids[atom_idx].clone())
        } else {
            Err(WorkspaceError::IndexOutOfCoreSize)
        }
    }

    pub fn idx_of_name(&self, name: &String) -> Result<usize, WorkspaceError> {
        if let Some(idx) = self
            .ids
            .iter()
            .position(|value| value.as_ref() == Some(name))
        {
            Ok(idx)
        } else {
            Err(WorkspaceError::NoSuchIdName)
        }
    }

    pub fn remove_name(&mut self, atom_idx: usize) -> Result<(), WorkspaceError> {
        if atom_idx < self.core_size {
            self.ids[atom_idx] = None;
            Ok(())
        } else {
            Err(WorkspaceError::IndexOutOfCoreSize)
        }
    }

    pub fn set_class_name<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (String, usize)>,
    {
        self.classes.extend(iter)
    }

    pub fn unset_class_name<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (String, usize)>,
    {
        for (class_name, atom_idx) in iter {
            self.classes.remove(&class_name, &atom_idx);
        }
    }

    pub fn remove_class(&mut self, class_name: &String) {
        self.classes.remove_left(class_name)
    }

    pub fn remove_atom_classes(&mut self, atom_idx: usize) {
        self.classes.remove_right(&atom_idx)
    }
}

#[derive(Serialize, Debug, Clone, Copy)]
pub enum WorkspaceError {
    StackNotFound,
    NoSuchIdName,
    IndexOutOfCoreSize,
}

mod test {
    #[test]
    fn rotation_around_point() {
        use std::f64::consts::PI;

        use nalgebra::{Matrix4, Point3, Transform3, Vector3};

        let p1 = Point3::new(0., 0., 0.);
        let p2 = Point3::new(0., 0., 1.);
        let rotation = Matrix4::new_rotation_wrt_point(PI / 2. * Vector3::new(1., 0., 0.), p2);
        let rotation = Transform3::from_matrix_unchecked(rotation);
        println!("{:#?}", rotation * p1);
        println!("{:#?}", rotation * p2);
    }
}
