use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oxidenes::rendering::*;

fn make_test_input() -> Vec<u32> {
    (0..256 * 240)
        .map(|i| {
            let r = ((i * 7) % 256) as u32;
            let g = ((i * 13) % 256) as u32;
            let b = ((i * 23) % 256) as u32;
            (r << 16) | (g << 8) | b
        })
        .collect()
}

fn bench_crt_filter(c: &mut Criterion) {
    let input = make_test_input();
    let mut output = vec![0u32; SCREEN_W * SCREEN_H];
    let sv_table = vec![200u8; SCREEN_W * SCREEN_H];
    let distortion_table: Vec<(u32, u32)> = (0..SCREEN_W * SCREEN_H)
        .map(|i| {
            let x = i % SCREEN_W;
            let y = i / SCREEN_W;
            let src_x = (x * 256 / SCREEN_W) as u32;
            let src_y = (y * 240 / SCREEN_H) as u32;
            ((src_x << 8) | 128, (src_y << 8) | 128)
        })
        .collect();
    let mask_table = vec![(256u16, 256u16, 256u16); SCREEN_W * SCREEN_H];
    let crt_cfg = CrtConfig::default();

    c.bench_function("crt_filter", |b| {
        b.iter(|| {
            crt_filter(
                black_box(&input),
                black_box(&mut output),
                black_box(&sv_table),
                black_box(&distortion_table),
                black_box(&crt_cfg),
                black_box(&mask_table),
                0,
                0,
            );
        })
    });
}

fn bench_phosphor_bloom(c: &mut Criterion) {
    let mut buffer: Vec<u32> = (0..SCREEN_W * SCREEN_H)
        .map(|i| {
            let v = ((i * 7) % 256) as u32;
            (v << 16) | (v << 8) | v
        })
        .collect();

    c.bench_function("phosphor_bloom", |b| {
        b.iter(|| {
            apply_phosphor_bloom(black_box(&mut buffer), SCREEN_W, SCREEN_H, 50);
        })
    });
}

fn bench_scanline_glow(c: &mut Criterion) {
    let mut buffer: Vec<u32> = (0..SCREEN_W * SCREEN_H)
        .map(|i| {
            let v = ((i * 7) % 256) as u32;
            (v << 16) | (v << 8) | v
        })
        .collect();

    c.bench_function("scanline_glow", |b| {
        b.iter(|| {
            apply_scanline_glow(black_box(&mut buffer), SCREEN_W, SCREEN_H, 50);
        })
    });
}

criterion_group!(
    benches,
    bench_crt_filter,
    bench_phosphor_bloom,
    bench_scanline_glow
);
criterion_main!(benches);
