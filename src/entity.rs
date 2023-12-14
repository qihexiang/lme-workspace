use std::{
    collections::HashMap,
    io::Write,
    process::{Command, Stdio},
};

use n_to_n::NtoN;
use nalgebra::{Point3, Transform3};
use petgraph::graphmap::UnGraphMap;
use rayon::iter::{
    IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelBridge, ParallelIterator,
};
use serde::{Deserialize, Serialize};

use crate::config::PLUGIN_DIRECTORY;

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub struct Atom {
    element: usize,
    position: Point3<f64>,
}

impl Atom {
    pub fn new(element: usize, position: Point3<f64>) -> Self {
        Self { element, position }
    }

    pub fn get_element(&self) -> usize {
        self.element
    }

    pub fn get_position(&self) -> Point3<f64> {
        self.position
    }

    pub fn set_element(self, element: usize) -> Self {
        Self { element, ..self }
    }

    pub fn set_position(self, position: Point3<f64>) -> Self {
        Self { position, ..self }
    }

    pub fn transform_position(self, transform: &Transform3<f64>) -> Self {
        Self {
            position: transform * self.position,
            ..self
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
pub struct Atoms(HashMap<usize, Option<Atom>>);

impl Atoms {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn data(&self) -> &HashMap<usize, Option<Atom>> {
        &self.0
    }

    fn data_mut(&mut self) -> &mut HashMap<usize, Option<Atom>> {
        &mut self.0
    }

    fn modify_atoms<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&mut HashMap<usize, Option<Atom>>),
    {
        f(self.data_mut());
        self
    }

    pub fn get(&self, idx: usize) -> Option<Atom> {
        self.data().get(&idx).copied().flatten()
    }

    pub fn set(&mut self, atoms: &Vec<(usize, Option<Atom>)>) -> &mut Self {
        self.modify_atoms(|map| {
            map.extend(atoms.clone());
        })
    }

    pub fn transform_all(&mut self, transform: &Transform3<f64>) -> &mut Self {
        self.modify_atoms(|map| {
            map.par_iter_mut().for_each(|(_, atom)| {
                if let Some(atom) = atom {
                    *atom = atom.transform_position(transform)
                }
            });
        })
    }

    pub fn overlay_to(&self, lower: &Self) -> Self {
        let mut merged = lower.data().clone();
        merged.extend(self.data());
        Self(merged)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Bonds(UnGraphMap<usize, Option<f64>>);

impl Bonds {
    pub fn new() -> Self {
        Self(UnGraphMap::new())
    }

    pub fn data(&self) -> &UnGraphMap<usize, Option<f64>> {
        &self.0
    }

    pub fn neighbors(&self, atom: usize) -> Vec<(usize, f64)> {
        let graph = self.data();

        graph
            .neighbors(atom)
            .par_bridge()
            .filter_map(|neighbor| {
                graph
                    .edge_weight(atom, neighbor)
                    .and_then(|bond| bond.map(|bond| (neighbor, bond)))
            })
            .collect()
    }

    fn modify_bonds<F>(&mut self, mut f: F) -> &mut Self
    where
        F: FnMut(&mut Self),
    {
        f(self);
        self
    }

    pub fn set_bonds(&mut self, bonds: &Vec<(usize, usize, Option<f64>)>) -> &mut Self {
        self.modify_bonds(|Self(graph)| {
            graph.extend(bonds);
        })
    }

    pub fn remove_atoms(&mut self, atoms: &Vec<usize>) -> &mut Self {
        let bonds = atoms
            .par_iter()
            .map(|atom| {
                self.data()
                    .neighbors(*atom)
                    .map(|neighbor| (*atom, neighbor, None as Option<f64>))
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();
        self.set_bonds(&bonds)
    }

    pub fn overlay_to(&self, lower: &Self) -> Self {
        let mut graph = lower.data().clone();
        graph.extend(self.data().all_edges());
        Self(graph)
    }
}

#[test]
fn merge_bonds() {
    let mut lower = Bonds::new();
    lower.set_bonds(&vec![(0, 1, Some(1.0)), (0, 2, Some(3.0))]);
    let mut middle = Bonds::new();
    middle.set_bonds(&vec![(3, 4, Some(1.0)), (4, 5, Some(3.0))]);
    let mut upper = Bonds::new();
    upper.set_bonds(&vec![(0, 1, None), (3, 4, None), (0, 4, Some(1.0))]);
    let merged = upper.overlay_to(&middle.overlay_to(&lower));
    for (a, b, bond) in merged.data().all_edges() {
        println!("{} {} {:?}", a, b, bond);
    }
    println!("{}", serde_json::to_string(&merged).unwrap())
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Molecule {
    atoms: Atoms,
    bonds: Bonds,
    pub classes: NtoN<String, usize>,
}

impl Molecule {
    pub fn overlay_to(
        &self,
        Molecule {
            atoms,
            bonds,
            classes,
        }: &Molecule,
    ) -> Molecule {
        let mut molecule = self.clone();
        molecule.atoms = molecule.atoms.overlay_to(atoms);
        molecule.bonds = molecule.bonds.overlay_to(bonds);
        molecule.classes = molecule.classes.overlay_to(classes);
        molecule
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Layer {
    Base(usize),
    Fill(Molecule),
    Transform(Transform3<f64>),
    HideBonds,
    Plugin {
        name: String,
        arguments: Vec<String>,
    },
}

impl Default for Layer {
    fn default() -> Self {
        Self::Base(0)
    }
}

impl Layer {
    pub fn read(&self, lower: &Molecule) -> Molecule {
        match self {
            Self::Base(size) => {
                let mut molecule = Molecule::default();
                let placeholders = (0..*size).map(|idx| (idx, None)).collect::<Vec<_>>();
                molecule.atoms.set(&placeholders);
                molecule.classes.extend((0..*size).map(|idx| (String::from("core"), idx)));
                molecule
            }
            Self::Fill(current) => current.overlay_to(lower),
            Self::HideBonds => {
                let mut molecule = lower.clone();
                molecule.bonds = Bonds::new();
                molecule
            }
            Self::Transform(transform) => {
                let mut molecule = lower.clone();
                molecule.atoms.transform_all(transform);
                molecule
            }
            Self::Plugin { name, arguments } => {
                let full_path = PLUGIN_DIRECTORY.join(name);
                let mut process = Command::new(full_path)
                    .args(arguments)
                    .stdin(Stdio::piped())
                    .spawn()
                    .unwrap();
                let mut stdin = process.stdin.take().unwrap();
                stdin
                    .write_all(serde_json::to_string(lower).unwrap().as_bytes())
                    .unwrap();
                let result = process.wait_with_output().unwrap();
                let molecule: Molecule = serde_json::from_slice(&result.stdout).unwrap();
                molecule
            }
        }
    }
}
