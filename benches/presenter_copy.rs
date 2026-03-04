use std::{hint::black_box, time::Duration};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

const BYTES_PER_PIXEL_RGB: usize = 3;
const BYTES_PER_PIXEL_RGBA: usize = 4;
const ALPHA_OPAQUE: u8 = 255;

struct BenchParams {
    label: &'static str,
    width: usize,
    height: usize,
}

const SCENARIOS: &[BenchParams] = &[
    BenchParams {
        label: "800x600",
        width: 800,
        height: 600,
    },
    BenchParams {
        label: "1920x1080",
        width: 1920,
        height: 1080,
    },
];

fn copy_rgba_into_frame(src: &[u8], dest: &mut [u8], width: usize, height: usize) {
    let expected_rgba_len = width * height * BYTES_PER_PIXEL_RGBA;

    assert_eq!(
        src.len(),
        expected_rgba_len,
        "pixel buffer length {} does not match expected {} for {}x{}",
        src.len(),
        expected_rgba_len,
        width,
        height
    );

    assert_eq!(
        dest.len(),
        expected_rgba_len,
        "pixels frame length {} does not match expected {} for {}x{}",
        dest.len(),
        expected_rgba_len,
        width,
        height
    );

    dest.copy_from_slice(src);
}

fn expand_rgb_into_frame(src: &[u8], dest: &mut [u8], width: usize, height: usize) {
    let expected_rgba_len = width * height * BYTES_PER_PIXEL_RGBA;

    assert_eq!(
        dest.len(),
        expected_rgba_len,
        "pixels frame length {} does not match expected {} for {}x{}",
        dest.len(),
        expected_rgba_len,
        width,
        height
    );

    for (src_pixel, dst_pixel) in src
        .chunks_exact(BYTES_PER_PIXEL_RGB)
        .zip(dest.chunks_exact_mut(BYTES_PER_PIXEL_RGBA))
    {
        dst_pixel[0] = src_pixel[0];
        dst_pixel[1] = src_pixel[1];
        dst_pixel[2] = src_pixel[2];
        dst_pixel[3] = ALPHA_OPAQUE;
    }
}

fn make_rgba_source(pixel_count: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(pixel_count * BYTES_PER_PIXEL_RGBA);
    for i in 0..pixel_count {
        let v = (i % 251) as u8;
        buffer.push(v);
        buffer.push(v.wrapping_add(1));
        buffer.push(v.wrapping_add(2));
        buffer.push(ALPHA_OPAQUE);
    }
    buffer
}

fn make_rgb_source(pixel_count: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(pixel_count * BYTES_PER_PIXEL_RGB);
    for i in 0..pixel_count {
        let v = (i % 251) as u8;
        buffer.push(v);
        buffer.push(v.wrapping_add(1));
        buffer.push(v.wrapping_add(2));
    }
    buffer
}

fn bench_presenter_copy(c: &mut Criterion) {
    let mut group = c.benchmark_group("presenter_copy");

    for params in SCENARIOS {
        let pixel_count = params.width * params.height;
        group.throughput(Throughput::Elements(pixel_count as u64));

        let src_rgba = make_rgba_source(pixel_count);
        let mut dest_rgba = vec![0_u8; pixel_count * BYTES_PER_PIXEL_RGBA];
        group.bench_with_input(
            BenchmarkId::new("copy_rgba_into_frame", params.label),
            params,
            |b, p| {
                b.iter(|| {
                    copy_rgba_into_frame(
                        black_box(src_rgba.as_slice()),
                        black_box(dest_rgba.as_mut_slice()),
                        p.width,
                        p.height,
                    );
                    black_box(dest_rgba[dest_rgba.len() - 1]);
                });
            },
        );

        let src_rgb = make_rgb_source(pixel_count);
        let mut dest_from_rgb = vec![0_u8; pixel_count * BYTES_PER_PIXEL_RGBA];
        group.bench_with_input(
            BenchmarkId::new("expand_rgb_into_frame", params.label),
            params,
            |b, p| {
                b.iter(|| {
                    expand_rgb_into_frame(
                        black_box(src_rgb.as_slice()),
                        black_box(dest_from_rgb.as_mut_slice()),
                        p.width,
                        p.height,
                    );
                    black_box(dest_from_rgb[dest_from_rgb.len() - 1]);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(5));
    targets = bench_presenter_copy,
);
criterion_main!(benches);
