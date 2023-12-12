use std::sync::Arc;

use n_to_n::NtoN;

pub mod entity;
pub mod config;

pub struct Stack;

pub enum Layer {
    Base(usize),
    Fill,
    Translation,
    Rotation,
    Plugin,
    ReplaceElement,
    HideElement,
    HideBonds,
}

pub struct Workspace {
    stacks: Vec<Arc<Stack>>,
    layers: Vec<Arc<Layer>>,
    ids: Vec<Option<String>>,
    classes: NtoN<usize, String>
}

mod test {
    use std::f64::consts::PI;

    use nalgebra::{Point3, Matrix4, Vector3, Transform3};

    #[test]
    fn rotation_around_point() {
        let p1 = Point3::new(0., 0., 0.);
        let p2 = Point3::new(0., 0., 1.);
        let rotation = Matrix4::new_rotation_wrt_point(PI / 2. * Vector3::new(1., 0., 0.), p2);
        let rotation = Transform3::from_matrix_unchecked(rotation);
        println!("{:#?}", rotation * p1);
        println!("{:#?}", rotation * p2);
    }
}
