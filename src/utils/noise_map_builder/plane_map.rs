use alloc::boxed::Box;

use crate::{math::interpolate, utils::NoiseMap, NoiseFn};

use super::{NoiseFnWrapper, NoiseMapBuilder};

/// The callback function triggered when there is an update on progress
///
/// # Arguments
/// * `u64`: The total size of the process (width * height)
/// * `u64`: The current number of of entries processed
type ProgressCallbackFn = dyn Fn(usize, usize) + Sync + Send + 'static;

/// Callback function configuration to track build progress when running 
/// asynchronously. Granularity is a number between 1 and 10 that indicates
/// how often the callback function should be called (1 = for every point,
/// 10 = every 10 points)
pub struct ProgressCallbackConfig {
    callback: Box<ProgressCallbackFn>,
    granularity: u8,
}

pub struct PlaneMapBuilder<SourceModule, const DIM: usize>
where
    SourceModule: NoiseFn<f64, DIM>,
{
    is_seamless: bool,
    x_bounds: (f64, f64),
    y_bounds: (f64, f64),
    size: (usize, usize),
    source_module: SourceModule,
    callback_config: Option<ProgressCallbackConfig>,
}

impl<SourceModule, const DIM: usize> PlaneMapBuilder<SourceModule, DIM>
where
    SourceModule: NoiseFn<f64, DIM>,
{
    pub fn new(source_module: SourceModule) -> Self {
        PlaneMapBuilder {
            is_seamless: false,
            x_bounds: (-1.0, 1.0),
            y_bounds: (-1.0, 1.0),
            size: (100, 100),
            source_module,
            callback_config: None,
        }
    }

    pub fn set_is_seamless(self, is_seamless: bool) -> Self {
        PlaneMapBuilder {
            is_seamless,
            ..self
        }
    }

    pub fn set_x_bounds(self, lower_x_bound: f64, upper_x_bound: f64) -> Self {
        PlaneMapBuilder {
            x_bounds: (lower_x_bound, upper_x_bound),
            ..self
        }
    }

    pub fn set_y_bounds(self, lower_y_bound: f64, upper_y_bound: f64) -> Self {
        PlaneMapBuilder {
            y_bounds: (lower_y_bound, upper_y_bound),
            ..self
        }
    }

    pub fn set_progress_callback(
        self,
        granularity: u8,
        callback: impl Fn(usize, usize) + Sync + Send + 'static,
    ) -> Self {
        let final_granularity = if granularity > 10 {
            granularity.checked_ilog10().unwrap_or(0) + 1
        } else {
            granularity as u32
        };

        PlaneMapBuilder {
            callback_config: Some(ProgressCallbackConfig {
                callback: Box::new(callback),
                granularity: final_granularity as u8,
            }),
            ..self
        }
    }

    pub fn x_bounds(&self) -> (f64, f64) {
        self.x_bounds
    }

    pub fn y_bounds(&self) -> (f64, f64) {
        self.y_bounds
    }
}

impl<SourceModule> NoiseMapBuilder<SourceModule> for PlaneMapBuilder<SourceModule, 3>
where
    SourceModule: NoiseFn<f64, 3>,
{
    fn set_size(self, width: usize, height: usize) -> Self {
        PlaneMapBuilder {
            size: (width, height),
            ..self
        }
    }

    fn set_source_module(self, source_module: SourceModule) -> Self {
        PlaneMapBuilder {
            source_module,
            ..self
        }
    }

    fn size(&self) -> (usize, usize) {
        self.size
    }

    fn build(&self) -> NoiseMap {
        let (width, height) = self.size;

        let mut result_map = NoiseMap::new(width, height);

        let x_extent = self.x_bounds.1 - self.x_bounds.0;
        let y_extent = self.y_bounds.1 - self.y_bounds.0;

        let x_step = x_extent / width as f64;
        let y_step = y_extent / height as f64;

        for y in 0..height {
            let current_y = self.y_bounds.0 + y_step * y as f64;

            for x in 0..width {
                let current_x = self.x_bounds.0 + x_step * x as f64;

                let final_value = if self.is_seamless {
                    let sw_value = self.source_module.get([current_x, current_y, 0.0]);
                    let se_value = self
                        .source_module
                        .get([current_x + x_extent, current_y, 0.0]);
                    let nw_value = self
                        .source_module
                        .get([current_x, current_y + y_extent, 0.0]);
                    let ne_value =
                        self.source_module
                            .get([current_x + x_extent, current_y + y_extent, 0.0]);

                    let x_blend = 1.0 - ((current_x - self.x_bounds.0) / x_extent);
                    let y_blend = 1.0 - ((current_y - self.y_bounds.0) / y_extent);

                    let y0 = interpolate::linear(sw_value, se_value, x_blend);
                    let y1 = interpolate::linear(nw_value, ne_value, x_blend);

                    interpolate::linear(y0, y1, y_blend)
                } else {
                    self.source_module.get([current_x, current_y, 0.0])
                };

                result_map[(x, y)] = final_value;
                if let Some(callback_config) = &self.callback_config {
                    let progress_pt = y * x;
                    if progress_pt % callback_config.granularity as usize == 0 {
                        callback_config.callback.as_ref()(width * height, progress_pt);
                    }
                }
            }
        }

        result_map
    }
}

impl<SourceFn, const DIM: usize> PlaneMapBuilder<NoiseFnWrapper<SourceFn, DIM>, DIM>
where
    SourceFn: Fn([f64; DIM]) -> f64,
{
    pub fn new_fn(source_fn: SourceFn) -> Self {
        PlaneMapBuilder {
            is_seamless: false,
            x_bounds: (-1.0, 1.0),
            y_bounds: (-1.0, 1.0),
            size: (100, 100),
            source_module: NoiseFnWrapper { source_fn },
            callback_config: None,
        }
    }

    pub fn set_size(self, width: usize, height: usize) -> Self {
        PlaneMapBuilder {
            size: (width, height),
            ..self
        }
    }
}

impl<SourceFn> PlaneMapBuilder<NoiseFnWrapper<SourceFn, 2>, 2>
where
    SourceFn: Fn([f64; 2]) -> f64,
{
    pub fn build(&self) -> NoiseMap {
        let (width, height) = self.size;

        let mut result_map = NoiseMap::new(width, height);

        let x_extent = self.x_bounds.1 - self.x_bounds.0;
        let y_extent = self.y_bounds.1 - self.y_bounds.0;

        let x_step = x_extent / width as f64;
        let y_step = y_extent / height as f64;

        for y in 0..height {
            let current_y = self.y_bounds.0 + y_step * y as f64;

            for x in 0..width {
                let current_x = self.x_bounds.0 + x_step * x as f64;

                let final_value = if self.is_seamless {
                    let sw_value = self.source_module.get([current_x, current_y]);
                    let se_value = self.source_module.get([current_x + x_extent, current_y]);
                    let nw_value = self.source_module.get([current_x, current_y + y_extent]);
                    let ne_value = self
                        .source_module
                        .get([current_x + x_extent, current_y + y_extent]);

                    let x_blend = 1.0 - ((current_x - self.x_bounds.0) / x_extent);
                    let y_blend = 1.0 - ((current_y - self.y_bounds.0) / y_extent);

                    let y0 = interpolate::linear(sw_value, se_value, x_blend);
                    let y1 = interpolate::linear(nw_value, ne_value, x_blend);

                    interpolate::linear(y0, y1, y_blend)
                } else {
                    self.source_module.get([current_x, current_y])
                };

                result_map[(x, y)] = final_value;
                if let Some(callback_config) = &self.callback_config {
                    let progress_pt = y * x;
                    if progress_pt % callback_config.granularity as usize == 0 {
                        callback_config.callback.as_ref()(width * height, progress_pt);
                    }
                }
            }
        }

        result_map
    }
}

impl<SourceFn> PlaneMapBuilder<NoiseFnWrapper<SourceFn, 3>, 3>
where
    SourceFn: Fn([f64; 3]) -> f64,
{
    pub fn build(&self) -> NoiseMap {
        let (width, height) = self.size;

        let mut result_map = NoiseMap::new(width, height);

        let x_extent = self.x_bounds.1 - self.x_bounds.0;
        let y_extent = self.y_bounds.1 - self.y_bounds.0;

        let x_step = x_extent / width as f64;
        let y_step = y_extent / height as f64;

        for y in 0..height {
            let current_y = self.y_bounds.0 + y_step * y as f64;

            for x in 0..width {
                let current_x = self.x_bounds.0 + x_step * x as f64;

                let final_value = if self.is_seamless {
                    let sw_value = self.source_module.get([current_x, current_y, 0.0]);
                    let se_value = self
                        .source_module
                        .get([current_x + x_extent, current_y, 0.0]);
                    let nw_value = self
                        .source_module
                        .get([current_x, current_y + y_extent, 0.0]);
                    let ne_value =
                        self.source_module
                            .get([current_x + x_extent, current_y + y_extent, 0.0]);

                    let x_blend = 1.0 - ((current_x - self.x_bounds.0) / x_extent);
                    let y_blend = 1.0 - ((current_y - self.y_bounds.0) / y_extent);

                    let y0 = interpolate::linear(sw_value, se_value, x_blend);
                    let y1 = interpolate::linear(nw_value, ne_value, x_blend);

                    interpolate::linear(y0, y1, y_blend)
                } else {
                    self.source_module.get([current_x, current_y, 0.0])
                };

                result_map[(x, y)] = final_value;
                if let Some(callback_config) = &self.callback_config {
                    let progress_pt = y * x;
                    if progress_pt % callback_config.granularity as usize == 0 {
                        callback_config.callback.as_ref()(width * height, progress_pt);
                    }
                }
            }
        }

        result_map
    }
}

impl<SourceFn> PlaneMapBuilder<NoiseFnWrapper<SourceFn, 4>, 4>
where
    SourceFn: Fn([f64; 4]) -> f64,
{
    pub fn build(&self) -> NoiseMap {
        let (width, height) = self.size;

        let mut result_map = NoiseMap::new(width, height);

        let x_extent = self.x_bounds.1 - self.x_bounds.0;
        let y_extent = self.y_bounds.1 - self.y_bounds.0;

        let x_step = x_extent / width as f64;
        let y_step = y_extent / height as f64;

        for y in 0..height {
            let current_y = self.y_bounds.0 + y_step * y as f64;

            for x in 0..width {
                let current_x = self.x_bounds.0 + x_step * x as f64;

                let final_value = if self.is_seamless {
                    let sw_value = self.source_module.get([current_x, current_y, 0.0, 0.5]);
                    let se_value =
                        self.source_module
                            .get([current_x + x_extent, current_y, 0.0, 0.5]);
                    let nw_value =
                        self.source_module
                            .get([current_x, current_y + y_extent, 0.0, 0.5]);
                    let ne_value = self.source_module.get([
                        current_x + x_extent,
                        current_y + y_extent,
                        0.0,
                        0.5,
                    ]);

                    let x_blend = 1.0 - ((current_x - self.x_bounds.0) / x_extent);
                    let y_blend = 1.0 - ((current_y - self.y_bounds.0) / y_extent);

                    let y0 = interpolate::linear(sw_value, se_value, x_blend);
                    let y1 = interpolate::linear(nw_value, ne_value, x_blend);

                    interpolate::linear(y0, y1, y_blend)
                } else {
                    self.source_module.get([current_x, current_y, 0.0, 0.5])
                };

                result_map[(x, y)] = final_value;
                if let Some(callback_config) = &self.callback_config {
                    let progress_pt = y * x;
                    if progress_pt % callback_config.granularity as usize == 0 {
                        callback_config.callback.as_ref()(width * height, progress_pt);
                    }
                }
            }
        }

        result_map
    }
}
