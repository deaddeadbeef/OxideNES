use criterion::{criterion_group, criterion_main, Criterion, black_box};
use oxidenes::rendering::*;

fn bench_glass_inner_loop(c: &mut Criterion) {
    let window_width = SCREEN_W + 2 * SCREEN_X;
    let mut buffer = vec![0x808080u32; window_width * (SCREEN_H + 2 * SCREEN_Y)];
    let ghost_source = vec![0x404040u32; SCREEN_W * SCREEN_H];
    let glare_table = vec![30u8; SCREEN_W * SCREEN_H];
    let thickness_table = vec![128u16; SCREEN_W * SCREEN_H];
    let ghost_alpha_table = vec![20u8; SCREEN_W * SCREEN_H];

    c.bench_function("glass_inner_loop", |b| {
        b.iter(|| {
            glass_inner_loop(
                black_box(&mut buffer),
                black_box(&ghost_source),
                black_box(&glare_table),
                black_box(&thickness_table),
                black_box(&ghost_alpha_table),
                window_width,
                50, 30, true,
                SCREEN_W - 10, SCREEN_H - 10,
                5, 5,
                SCREEN_H - 10, SCREEN_W - 10,
                SCREEN_W,
            );
        })
    });
}

criterion_group!(benches, bench_glass_inner_loop);
criterion_main!(benches);
