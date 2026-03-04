use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

use fractal_explorer::core::{
    actions::{
        generate_fractal::generate_fractal_parallel_rayon::generate_fractal_parallel_rayon,
        generate_pixel_buffer::generate_pixel_buffer::generate_pixel_buffer,
    },
    data::{complex::Complex, complex_rect::ComplexRect, pixel_rect::PixelRect, point::Point},
    fractals::mandelbrot::{
        algorithm::MandelbrotAlgorithm,
        colour_mapping::{factory::mandelbrot_colour_map_factory, kinds::MandelbrotColourMapKinds},
    },
};

struct BenchParams {
    label: &'static str,
    width: i32,
    height: i32,
    max_iterations: u32,
}

const SCENARIOS: &[BenchParams] = &[
    BenchParams {
        label: "800x600/256iter",
        width: 800,
        height: 600,
        max_iterations: 256,
    },
    BenchParams {
        label: "1920x1080/256iter",
        width: 1920,
        height: 1080,
        max_iterations: 256,
    },
    BenchParams {
        label: "800x600/1024iter",
        width: 800,
        height: 600,
        max_iterations: 1024,
    },
];

/// Default Mandelbrot viewport: full set view
const COMPLEX_TOP_LEFT: Complex = Complex {
    real: -2.5,
    imag: -1.0,
};
const COMPLEX_BOTTOM_RIGHT: Complex = Complex {
    real: 1.0,
    imag: 1.0,
};

fn bench_fractal_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("fractal_generation");

    for params in SCENARIOS {
        let pixel_count = (params.width as u64) * (params.height as u64);
        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: params.width - 1,
                y: params.height - 1,
            },
        )
        .unwrap();

        let complex_rect =
            ComplexRect::new(COMPLEX_TOP_LEFT, COMPLEX_BOTTOM_RIGHT).unwrap();

        let algorithm =
            MandelbrotAlgorithm::new(pixel_rect, complex_rect, params.max_iterations).unwrap();

        group.throughput(Throughput::Elements(pixel_count));
        group.bench_with_input(
            BenchmarkId::new("parallel_rayon", params.label),
            &algorithm,
            |b, alg| {
                b.iter_with_large_drop(|| generate_fractal_parallel_rayon(pixel_rect, alg).unwrap());
            },
        );
    }

    group.finish();
}

fn bench_colour_mapping(c: &mut Criterion) {
    let mut group = c.benchmark_group("colour_mapping");

    for params in SCENARIOS {
        let pixel_count = (params.width as u64) * (params.height as u64);
        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: params.width - 1,
                y: params.height - 1,
            },
        )
        .unwrap();

        let complex_rect =
            ComplexRect::new(COMPLEX_TOP_LEFT, COMPLEX_BOTTOM_RIGHT).unwrap();

        let algorithm =
            MandelbrotAlgorithm::new(pixel_rect, complex_rect, params.max_iterations).unwrap();

        // Pre-compute iterations once (we're benchmarking colour mapping, not fractal gen)
        let iterations = generate_fractal_parallel_rayon(pixel_rect, &algorithm).unwrap();

        let colour_map =
            mandelbrot_colour_map_factory(MandelbrotColourMapKinds::FireGradient, params.max_iterations);

        group.throughput(Throughput::Elements(pixel_count));
        group.bench_with_input(
            BenchmarkId::new("fire_gradient", params.label),
            &iterations,
            |b, iters: &Vec<u32>| {
                b.iter_batched(
                    || iters.clone(),
                    |input| generate_pixel_buffer(input, colour_map.as_ref(), pixel_rect).unwrap(),
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    for params in SCENARIOS {
        let pixel_count = (params.width as u64) * (params.height as u64);
        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: params.width - 1,
                y: params.height - 1,
            },
        )
        .unwrap();

        let complex_rect =
            ComplexRect::new(COMPLEX_TOP_LEFT, COMPLEX_BOTTOM_RIGHT).unwrap();

        let algorithm =
            MandelbrotAlgorithm::new(pixel_rect, complex_rect, params.max_iterations).unwrap();

        let colour_map =
            mandelbrot_colour_map_factory(MandelbrotColourMapKinds::FireGradient, params.max_iterations);

        group.throughput(Throughput::Elements(pixel_count));
        group.bench_with_input(
            BenchmarkId::new("generate_and_map", params.label),
            &(),
            |b, _| {
                b.iter_with_large_drop(|| {
                    let iterations =
                        generate_fractal_parallel_rayon(pixel_rect, &algorithm).unwrap();
                    generate_pixel_buffer(iterations, colour_map.as_ref(), pixel_rect).unwrap()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(15));
    targets = bench_fractal_generation,
    bench_colour_mapping,
    bench_full_pipeline,
);
criterion_main!(benches);
