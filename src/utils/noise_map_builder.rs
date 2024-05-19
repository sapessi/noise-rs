use crate::{noise_fns::NoiseFn, utils::noise_map::NoiseMap};

pub struct NoiseFnWrapper<SourceFn, const DIM: usize>
where
    SourceFn: Fn([f64; DIM]) -> f64,
{
    source_fn: SourceFn,
}

impl<F, const DIM: usize> NoiseFn<f64, DIM> for NoiseFnWrapper<F, DIM>
where
    F: Fn([f64; DIM]) -> f64,
{
    fn get(&self, point: [f64; DIM]) -> f64 {
        (self.source_fn)(point)
    }
}

pub trait NoiseMapBuilder<SourceModule> {
    fn set_size(self, width: usize, height: usize) -> Self;

    fn set_source_module(self, source_module: SourceModule) -> Self;

    fn size(&self) -> (usize, usize);

    fn build(&self) -> NoiseMap;
}

mod cylinder_map;
mod plane_map;
mod sphere_map;

pub use cylinder_map::*;
pub use plane_map::*;
pub use sphere_map::*;
