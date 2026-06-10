use crate::controllers::interactive::ports::gpu_renderer::GpuFractalRendererPort;
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::fractals::mandelbrot::perturbation::algorithm::MandelbrotPerturbationAlgorithm;
use wgpu::util::DeviceExt;

/// Below this view extent, f32 deltas underflow/lose too much precision and
/// the render falls back to the CPU's f64 delta iteration.
const MIN_SUPPORTED_EXTENT: f64 = 1e-30;

const WORKGROUP_SIZE: u32 = 16;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuParams {
    width: u32,
    height: u32,
    max_iterations: u32,
    orbit_len: u32,
    origin_re: f32,
    origin_im: f32,
    step_re: f32,
    step_im: f32,
}

struct GpuState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

/// wgpu compute implementation of the perturbation delta iteration.
///
/// The device is created lazily on first use (on the render worker thread);
/// if initialisation or any render step fails, the renderer reports
/// unavailability and the caller falls back to the CPU path.
pub struct WgpuPerturbationRenderer {
    state: Option<GpuState>,
    init_failed: bool,
}

impl WgpuPerturbationRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: None,
            init_failed: false,
        }
    }

    fn state(&mut self) -> Option<&GpuState> {
        if self.init_failed {
            return None;
        }

        if self.state.is_none() {
            match Self::init() {
                Some(state) => self.state = Some(state),
                None => {
                    self.init_failed = true;
                    eprintln!(
                        "GPU perturbation renderer unavailable; falling back to CPU rendering"
                    );
                    return None;
                }
            }
        }

        self.state.as_ref()
    }

    fn init() -> Option<GpuState> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("perturbation compute device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        ))
        .ok()?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("perturbation compute shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("perturbation.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("perturbation bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("perturbation pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("perturbation pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        Some(GpuState {
            device,
            queue,
            pipeline,
            bind_group_layout,
        })
    }

    fn render(
        state: &GpuState,
        algorithm: &MandelbrotPerturbationAlgorithm,
    ) -> Option<Vec<u32>> {
        let pixel_rect = algorithm.pixel_rect();
        let width = pixel_rect.width();
        let height = pixel_rect.height();

        if width == 0 || height == 0 {
            return None;
        }

        // The orbit is resolved by FractalConfig::prepare before rendering;
        // without it (or with a trivial orbit) the CPU path takes over.
        let orbit = algorithm.orbit()?;
        let orbit_values = orbit.orbit();
        if orbit_values.len() < 2 {
            return None;
        }

        let grid = algorithm.delta_grid();
        let params = GpuParams {
            width,
            height,
            max_iterations: algorithm.max_iterations(),
            orbit_len: orbit_values.len() as u32,
            origin_re: grid.origin_re as f32,
            origin_im: grid.origin_im as f32,
            step_re: grid.step_re as f32,
            step_im: grid.step_im as f32,
        };

        let orbit_f32: Vec<[f32; 2]> = orbit_values
            .iter()
            .map(|z| [z[0] as f32, z[1] as f32])
            .collect();

        let output_size = (width as u64) * (height as u64) * std::mem::size_of::<u32>() as u64;
        let max_binding = state.device.limits().max_storage_buffer_binding_size as u64;
        if output_size > max_binding {
            return None;
        }

        let params_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("perturbation params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let orbit_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("perturbation orbit"),
                contents: bytemuck::cast_slice(&orbit_f32),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perturbation output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perturbation staging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("perturbation bind group"),
            layout: &state.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: orbit_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("perturbation encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("perturbation pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&state.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(
                width.div_ceil(WORKGROUP_SIZE),
                height.div_ceil(WORKGROUP_SIZE),
                1,
            );
        }

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_size);
        state.queue.submit(std::iter::once(encoder.finish()));

        let slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        state.device.poll(wgpu::Maintain::Wait);

        match receiver.recv() {
            Ok(Ok(())) => {}
            _ => return None,
        }

        let iterations = bytemuck::cast_slice::<u8, u32>(&slice.get_mapped_range()).to_vec();
        staging_buffer.unmap();

        Some(iterations)
    }
}

impl Default for WgpuPerturbationRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuFractalRendererPort for WgpuPerturbationRenderer {
    fn render_iterations(
        &mut self,
        algorithm: &MandelbrotPerturbationAlgorithm,
    ) -> Option<Vec<u32>> {
        if algorithm.region().min_extent() < MIN_SUPPORTED_EXTENT {
            return None;
        }

        let state = self.state()?;
        Self::render(state, algorithm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::actions::cancellation::NeverCancel;
    use crate::core::data::deep_complex::DeepComplex;
    use crate::core::data::deep_region::DeepRegion;
    use crate::core::data::pixel_rect::PixelRect;
    use crate::core::data::point::Point;
    use crate::core::fractals::mandelbrot::perturbation::orbit_cache::OrbitCache;
    use std::sync::Arc;

    fn perturbation_algorithm(
        width: i32,
        height: i32,
        centre_re: f64,
        centre_im: f64,
        extent: f64,
        max_iterations: u32,
    ) -> MandelbrotPerturbationAlgorithm {
        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: width - 1,
                y: height - 1,
            },
        )
        .unwrap();
        let region = DeepRegion::new(
            DeepComplex::from_f64(centre_re, centre_im).unwrap(),
            extent,
            extent,
        )
        .unwrap()
        .normalised();

        let algorithm = MandelbrotPerturbationAlgorithm::new(
            pixel_rect,
            region,
            max_iterations,
            Arc::new(OrbitCache::new()),
        )
        .unwrap();
        algorithm.prepare(&NeverCancel).unwrap();
        algorithm
    }

    /// GPU tests are skipped (not failed) on machines with no usable
    /// adapter, e.g. headless CI.
    fn gpu_or_skip() -> Option<WgpuPerturbationRenderer> {
        let mut renderer = WgpuPerturbationRenderer::new();
        if renderer.state().is_none() {
            eprintln!("skipping GPU test: no wgpu adapter available");
            return None;
        }
        Some(renderer)
    }

    #[test]
    fn gpu_matches_cpu_iterations_at_deep_zoom() {
        let Some(mut renderer) = gpu_or_skip() else {
            return;
        };

        let (width, height) = (96, 64);
        let max_iterations = 400;
        let algorithm = perturbation_algorithm(
            width,
            height,
            -0.74364388703715,
            0.13182590420532,
            1e-12,
            max_iterations,
        );

        let gpu = renderer
            .render_iterations(&algorithm)
            .expect("GPU render should succeed");
        assert_eq!(gpu.len(), (width * height) as usize);

        let mut cpu = Vec::with_capacity(gpu.len());
        for y in 0..height {
            algorithm
                .compute_row_segment_into(y, 0, width - 1, &mut cpu)
                .unwrap();
        }

        let total = gpu.len();
        let mismatches = gpu.iter().zip(cpu.iter()).filter(|(g, c)| g != c).count();
        let mismatch_fraction = mismatches as f64 / total as f64;

        // f32 deltas round differently from f64; boundary pixels may land on
        // a neighbouring iteration count, but the images must agree almost
        // everywhere.
        assert!(
            mismatch_fraction < 0.02,
            "GPU diverged from CPU: {mismatches}/{total} pixels differ"
        );
    }

    #[test]
    fn declines_below_the_f32_extent_floor() {
        let Some(mut renderer) = gpu_or_skip() else {
            return;
        };

        let algorithm = perturbation_algorithm(8, 8, -2.0, 0.0, 1e-40, 100);

        assert!(renderer.render_iterations(&algorithm).is_none());
    }
}
