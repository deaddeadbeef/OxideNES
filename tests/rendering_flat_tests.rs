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

fn make_flat_distortion_table() -> Vec<(u32, u32)> {
    let mut table = Vec::with_capacity(SCREEN_W * SCREEN_H);
    for dst_y in 0..SCREEN_H {
        for dst_x in 0..SCREEN_W {
            let src_x = dst_x as f32 / SCREEN_W as f32 * 256.0;
            let src_y = dst_y as f32 / SCREEN_H as f32 * 240.0;
            let src_x = src_x.clamp(0.0, 255.98);
            let src_y = src_y.clamp(0.0, 239.98);
            table.push(((src_x * 256.0) as u32, (src_y * 256.0) as u32));
        }
    }
    table
}

fn make_sv_table() -> Vec<u8> {
    (0..SCREEN_W * SCREEN_H)
        .map(|i| 170 + (i % 67) as u8)
        .collect()
}

fn make_mask_table() -> Vec<(u16, u16, u16)> {
    (0..SCREEN_W * SCREEN_H)
        .map(|i| {
            let r = 220 + (i % 37) as u16;
            let g = 210 + (i % 43) as u16;
            let b = 205 + (i % 47) as u16;
            (r, g, b)
        })
        .collect()
}

fn make_phosphor_lut() -> [(u32, u32, u32); 256] {
    let mut lut = [(256u32, 256u32, 256u32); 256];
    for (brightness, entry) in lut.iter_mut().enumerate() {
        let brightness = brightness as u32;
        *entry = (
            256 + brightness * 18 / 255,
            256 - brightness * 7 / 255,
            256 - brightness * 24 / 255,
        );
    }
    lut
}

#[test]
fn flat_masked_path_matches_generic_masked_path() {
    let input = make_test_input();
    let sv_table = make_sv_table();
    let flat_table = make_flat_distortion_table();
    let mask_table = make_mask_table();
    let phosphor_lut = make_phosphor_lut();

    let mut generic = vec![0u32; SCREEN_W * SCREEN_H];
    let mut flat = vec![0u32; SCREEN_W * SCREEN_H];

    crt_filter_masked(
        &input,
        &mut generic,
        &sv_table,
        &flat_table,
        &phosphor_lut,
        &mask_table,
        &GAMMA_TABLE,
    );
    crt_filter_flat_masked(
        &input,
        &mut flat,
        &sv_table,
        &flat_table,
        &phosphor_lut,
        &mask_table,
        &GAMMA_TABLE,
    );

    assert_eq!(flat, generic);
}

#[test]
fn flat_basic_path_matches_generic_basic_path() {
    let input = make_test_input();
    let sv_table = make_sv_table();
    let flat_table = make_flat_distortion_table();
    let phosphor_lut = make_phosphor_lut();

    let mut generic = vec![0u32; SCREEN_W * SCREEN_H];
    let mut flat = vec![0u32; SCREEN_W * SCREEN_H];

    crt_filter_basic(
        &input,
        &mut generic,
        &sv_table,
        &flat_table,
        &phosphor_lut,
        &GAMMA_TABLE,
    );
    crt_filter_flat_basic(
        &input,
        &mut flat,
        &sv_table,
        &flat_table,
        &phosphor_lut,
        &GAMMA_TABLE,
    );

    assert_eq!(flat, generic);
}
