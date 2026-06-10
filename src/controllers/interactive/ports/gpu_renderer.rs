use crate::core::fractals::mandelbrot::perturbation::algorithm::MandelbrotPerturbationAlgorithm;

/// Driven port for offloading the perturbation delta iteration to a GPU.
///
/// Implementations return the per-pixel iteration counts in row-major order
/// for the algorithm's pixel rect, or `None` when the GPU path is
/// unavailable or unsuitable for the request (the caller then falls back to
/// the CPU path). The algorithm is expected to be `prepare`d — its reference
/// orbit already resolved — before this is called.
pub trait GpuFractalRendererPort: Send {
    fn render_iterations(
        &mut self,
        algorithm: &MandelbrotPerturbationAlgorithm,
    ) -> Option<Vec<u32>>;
}
