// Mandelbrot perturbation delta iteration with rebasing.
//
// Mirrors MandelbrotPerturbationAlgorithm::iterate_delta (f64 on the CPU)
// in f32: one invocation per pixel iterates its delta against the shared
// reference orbit and writes the escape iteration count.

struct Params {
    width: u32,
    height: u32,
    max_iterations: u32,
    orbit_len: u32,
    origin_re: f32,
    origin_im: f32,
    step_re: f32,
    step_im: f32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> orbit: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> iterations: array<u32>;

const ESCAPE_RADIUS_SQ: f32 = 4.0;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.width || gid.y >= params.height) {
        return;
    }

    let dc = vec2<f32>(
        params.origin_re + f32(gid.x) * params.step_re,
        params.origin_im + f32(gid.y) * params.step_im,
    );

    let last = params.orbit_len - 1u;
    var d = vec2<f32>(0.0, 0.0);
    var m = 0u;
    var result = params.max_iterations;

    for (var n = 1u; n <= params.max_iterations; n = n + 1u) {
        // The reference orbit ended (escaped reference): rebase the full
        // value onto the orbit start.
        if (m == last) {
            d = orbit[m] + d;
            m = 0u;
        }

        // delta' = (2*Z_m + delta)*delta + dc
        let s = 2.0 * orbit[m] + d;
        d = vec2<f32>(
            s.x * d.x - s.y * d.y + dc.x,
            s.x * d.y + s.y * d.x + dc.y,
        );
        m = m + 1u;

        let z = orbit[m] + d;
        let z_mag_sq = dot(z, z);

        if (z_mag_sq > ESCAPE_RADIUS_SQ) {
            result = n;
            break;
        }

        // Rebase when the full value drops below the delta (glitch
        // avoidance, Zhuoran 2021).
        if (z_mag_sq < dot(d, d)) {
            d = z;
            m = 0u;
        }
    }

    iterations[gid.y * params.width + gid.x] = result;
}
