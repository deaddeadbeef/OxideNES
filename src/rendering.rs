// Rendering pipeline: CRT filter, glass effects, and supporting helpers.
// Extracted from main.rs for modularity and benchmarking.

use serde::{Deserialize, Serialize};

// ── Screen geometry constants ────────────────────────────────────────────────
pub const SCREEN_W: usize = 960;
pub const SCREEN_H: usize = 720;
pub const SCREEN_X: usize = 70;
pub const SCREEN_Y: usize = 50;
/// Number of rows per processing chunk.
const PAR_ROWS: usize = 16;

// ── CRT mask mode ────────────────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum CrtMaskMode {
    Off,
    ShadowMask,
    ApertureGrille,
    #[default]
    SlotMask,
}

// ── CRT configuration ────────────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct CrtConfig {
    pub scanline_intensity: u8,
    pub phosphor_warmth: u8,
    pub vignette_strength: u8,
    pub blur_amount: u8,
    pub curvature_strength: u8,
    pub mask_mode: CrtMaskMode,
    pub mask_intensity: u8,
    pub brightness: i8,
    pub contrast: i8,
}

impl Default for CrtConfig {
    fn default() -> Self {
        Self {
            scanline_intensity: 40,
            phosphor_warmth: 30,
            vignette_strength: 20,
            blur_amount: 0,
            curvature_strength: 15,
            mask_mode: CrtMaskMode::SlotMask,
            mask_intensity: 50,
            brightness: 0,
            contrast: 0,
        }
    }
}

// ── Performance overlay ──────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PerfOverlayLevel {
    Off,
    Basic,
    Detailed,
}

impl PerfOverlayLevel {
    pub fn next(self) -> Self {
        match self {
            PerfOverlayLevel::Off => PerfOverlayLevel::Basic,
            PerfOverlayLevel::Basic => PerfOverlayLevel::Detailed,
            PerfOverlayLevel::Detailed => PerfOverlayLevel::Off,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct PerfSnapshot {
    pub crt_us: u32,
    pub bloom_us: u32,
    pub composite_us: u32,
    pub glass_us: u32,
}

pub fn should_reset_fps_on_transition(prev: PerfOverlayLevel, next: PerfOverlayLevel) -> bool {
    prev == PerfOverlayLevel::Off && next != PerfOverlayLevel::Off
}

pub fn should_prime_detail_sampling(prev: PerfOverlayLevel, next: PerfOverlayLevel) -> bool {
    prev != PerfOverlayLevel::Detailed && next == PerfOverlayLevel::Detailed
}

// ── Gamma table ──────────────────────────────────────────────────────────────

/// Integer square root for const context
const fn isqrt_const(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// CRT gamma lookup table — precomputed for γ2.2 (inverse gamma for display)
pub const GAMMA_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        let sq = isqrt_const(i as u32 * 255);
        let lin = i as u32;
        let val = (sq * 230 + lin * 26) / 256;
        table[i] = if val > 255 { 255 } else { val as u8 };
        i += 1;
    }
    table
};

// ── Inline helpers ───────────────────────────────────────────────────────────

#[inline(always)]
pub fn blend_bilinear_rgb(
    p00: u32,
    p10: u32,
    p01: u32,
    p11: u32,
    frac_x: u32,
    frac_y: u32,
) -> (u32, u32, u32) {
    let inv_fx = 256 - frac_x;
    let inv_fy = 256 - frac_y;

    let rb00 = p00 & 0x00FF00FF;
    let rb10 = p10 & 0x00FF00FF;
    let rb01 = p01 & 0x00FF00FF;
    let rb11 = p11 & 0x00FF00FF;

    let rb_top = ((rb00 * inv_fx + rb10 * frac_x) >> 8) & 0x00FF00FF;
    let rb_bot = ((rb01 * inv_fx + rb11 * frac_x) >> 8) & 0x00FF00FF;
    let rb = ((rb_top * inv_fy + rb_bot * frac_y) >> 8) & 0x00FF00FF;

    let g00 = (p00 >> 8) & 0xFF;
    let g10 = (p10 >> 8) & 0xFF;
    let g01 = (p01 >> 8) & 0xFF;
    let g11 = (p11 >> 8) & 0xFF;
    let g_top = g00 * inv_fx + g10 * frac_x;
    let g_bot = g01 * inv_fx + g11 * frac_x;
    let g = ((g_top * inv_fy + g_bot * frac_y) >> 16) & 0xFF;

    ((rb >> 16) & 0xFF, g, rb & 0xFF)
}

#[inline(always)]
pub fn apply_blur_3tap(
    r: u32,
    g: u32,
    b: u32,
    left: u32,
    right: u32,
    blur_center: u32,
    blur_side: u32,
) -> (u32, u32, u32) {
    let r =
        (r * blur_center + ((left >> 16) & 0xFF) * blur_side + ((right >> 16) & 0xFF) * blur_side)
            >> 8;
    let g =
        (g * blur_center + ((left >> 8) & 0xFF) * blur_side + ((right >> 8) & 0xFF) * blur_side)
            >> 8;
    let b = (b * blur_center + (left & 0xFF) * blur_side + (right & 0xFF) * blur_side) >> 8;
    (r, g, b)
}

#[allow(dead_code)]
#[inline(always)]
pub fn apply_scanline_vignette(r: u32, g: u32, b: u32, scan_mul: u32, vig: u32) -> (u32, u32, u32) {
    let combined = (scan_mul * vig) >> 8;
    (
        (r * combined) >> 8,
        (g * combined) >> 8,
        (b * combined) >> 8,
    )
}

#[inline(always)]
pub fn apply_mask(r: u32, g: u32, b: u32, mr: u16, mg: u16, mb: u16) -> (u32, u32, u32) {
    (
        (r * mr as u32) >> 8,
        (g * mg as u32) >> 8,
        (b * mb as u32) >> 8,
    )
}

#[inline(always)]
pub fn pack_rgb(r: u32, g: u32, b: u32) -> u32 {
    (r.min(255) << 16) | (g.min(255) << 8) | b.min(255)
}

#[inline(always)]
pub fn pack_rgb_unchecked(r: u32, g: u32, b: u32) -> u32 {
    (r << 16) | (g << 8) | b
}

#[inline(always)]
pub fn sq_dist(x1: usize, y1: usize, x2: usize, y2: usize) -> usize {
    let dx = x1.abs_diff(x2);
    let dy = y1.abs_diff(y2);
    dx * dx + dy * dy
}

#[inline]
fn build_gamma_lut(brightness: i32, contrast: i32) -> [u8; 256] {
    if brightness == 0 && contrast == 0 {
        return GAMMA_TABLE;
    }

    let mut lut = [0u8; 256];
    let con_scale = 256 + (contrast * 256 / 50);
    for i in 0..256 {
        let gamma_val = GAMMA_TABLE[i] as i32;
        let val = (((gamma_val - 128) * con_scale) >> 8) + 128 + brightness * 255 / 50;
        lut[i] = val.clamp(0, 255) as u8;
    }
    lut
}

#[inline]
fn build_phosphor_lut(phosphor_warmth: u32) -> [(u32, u32, u32); 256] {
    let pr_mul = 256 + (phosphor_warmth * 24 / 100);
    let pg_mul = 256 - (phosphor_warmth * 8 / 100);
    let pb_mul = 256 - (phosphor_warmth * 36 / 100);

    let mut lut = [(256u32, 256u32, 256u32); 256];
    if phosphor_warmth > 0 {
        let pr_delta = pr_mul as i32 - 256;
        let pg_delta = pg_mul as i32 - 256;
        let pb_delta = pb_mul as i32 - 256;
        for br in 0..256u32 {
            let bri = br as i32;
            lut[br as usize] = (
                (256 + ((pr_delta * bri) >> 8)) as u32,
                (256 + ((pg_delta * bri) >> 8)) as u32,
                (256 + ((pb_delta * bri) >> 8)) as u32,
            );
        }
    }
    lut
}

#[inline]
fn distortion_table_looks_flat(distortion_table: &[(u32, u32)]) -> bool {
    if distortion_table.len() < SCREEN_W * SCREEN_H {
        return false;
    }

    let top_left = unsafe { *distortion_table.get_unchecked(0) };
    if top_left.0 == 0xFFFFFFFF {
        return false;
    }

    let top_mid = unsafe { *distortion_table.get_unchecked(SCREEN_W / 2) };
    let mid_left = unsafe { *distortion_table.get_unchecked((SCREEN_H / 2) * SCREEN_W) };
    let bottom_left = unsafe { *distortion_table.get_unchecked((SCREEN_H - 1) * SCREEN_W) };
    let bottom_mid =
        unsafe { *distortion_table.get_unchecked((SCREEN_H - 1) * SCREEN_W + SCREEN_W / 2) };

    top_mid.1 == top_left.1 && mid_left.0 == top_left.0 && bottom_mid.1 == bottom_left.1
}

#[inline(always)]
fn scale_flat_source_row(
    input: &[u32],
    x_map: &[(u32, u32)],
    src_y: usize,
    row_rb: &mut [u32; SCREEN_W],
    row_g: &mut [u32; SCREEN_W],
) {
    let base_offset = src_y * 256;

    for dst_x in 0..SCREEN_W {
        let src_xf = unsafe { x_map.get_unchecked(dst_x).0 };
        let src_x0 = (src_xf >> 8) as usize;
        let src_x1 = if src_x0 < 255 { src_x0 + 1 } else { 255 };
        let frac_x = if src_x0 >= 255 { 0 } else { src_xf & 0xFF };

        let p0 = unsafe { *input.get_unchecked(base_offset + src_x0) };
        if frac_x == 0 {
            unsafe {
                *row_rb.get_unchecked_mut(dst_x) = p0 & 0x00FF00FF;
                *row_g.get_unchecked_mut(dst_x) = ((p0 >> 8) & 0xFF) << 8;
            }
        } else {
            let p1 = unsafe { *input.get_unchecked(base_offset + src_x1) };
            let inv_fx = 256 - frac_x;
            let rb = (((p0 & 0x00FF00FF) * inv_fx + (p1 & 0x00FF00FF) * frac_x) >> 8) & 0x00FF00FF;
            let g = ((p0 >> 8) & 0xFF) * inv_fx + ((p1 >> 8) & 0xFF) * frac_x;
            unsafe {
                *row_rb.get_unchecked_mut(dst_x) = rb;
                *row_g.get_unchecked_mut(dst_x) = g;
            }
        }
    }
}

#[inline(always)]
fn blend_flat_rows_rgb(
    top_rb: u32,
    bot_rb: u32,
    top_g: u32,
    bot_g: u32,
    frac_y: u32,
) -> (u32, u32, u32) {
    if frac_y == 0 {
        return ((top_rb >> 16) & 0xFF, (top_g >> 8) & 0xFF, top_rb & 0xFF);
    }

    let inv_fy = 256 - frac_y;
    let rb = ((top_rb * inv_fy + bot_rb * frac_y) >> 8) & 0x00FF00FF;
    let g = ((top_g * inv_fy + bot_g * frac_y) >> 16) & 0xFF;
    ((rb >> 16) & 0xFF, g, rb & 0xFF)
}

// ── Phosphor bloom ───────────────────────────────────────────────────────────

#[inline]
pub fn apply_phosphor_bloom(buffer: &mut [u32], width: usize, height: usize, bloom_strength: u32) {
    if bloom_strength == 0 {
        return;
    }

    let threshold: u32 = 180;
    let bleed = (bloom_strength * 16 / 100).min(15);
    if bleed == 0 {
        return;
    }

    for y in 0..height {
        let row = y * width;

        let mut carry_r: u32 = 0;
        let mut carry_g: u32 = 0;
        let mut carry_b: u32 = 0;

        for x in 0..width {
            let idx = row + x;
            let pixel = unsafe { *buffer.get_unchecked(idx) };
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;

            if carry_r | carry_g | carry_b != 0 {
                let nr = (r + carry_r).min(255);
                let ng = (g + carry_g).min(255);
                let nb = (b + carry_b).min(255);
                unsafe {
                    *buffer.get_unchecked_mut(idx) = (nr << 16) | (ng << 8) | nb;
                }
                carry_r >>= 1;
                carry_g >>= 1;
                carry_b >>= 1;
            }

            let brightness = ((r + g + b) * 85) >> 8;
            if brightness > threshold {
                let excess = brightness - threshold;
                carry_r = (r * excess * bleed) >> 16;
                carry_g = (g * excess * bleed) >> 16;
                carry_b = (b * excess * bleed) >> 16;
            }
        }

        carry_r = 0;
        carry_g = 0;
        carry_b = 0;

        for x in (0..width).rev() {
            let idx = row + x;
            let pixel = unsafe { *buffer.get_unchecked(idx) };
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;

            if carry_r | carry_g | carry_b != 0 {
                let nr = (r + carry_r).min(255);
                let ng = (g + carry_g).min(255);
                let nb = (b + carry_b).min(255);
                unsafe {
                    *buffer.get_unchecked_mut(idx) = (nr << 16) | (ng << 8) | nb;
                }
                carry_r >>= 2;
                carry_g >>= 2;
                carry_b >>= 2;
            }

            let brightness = ((r + g + b) * 85) >> 8;
            if brightness > threshold {
                let excess = brightness - threshold;
                carry_r = (r * excess * bleed) >> 16;
                carry_g = (g * excess * bleed) >> 16;
                carry_b = (b * excess * bleed) >> 16;
            }
        }
    }
}

// ── Scanline glow ────────────────────────────────────────────────────────────

#[inline]
pub fn apply_scanline_glow(buffer: &mut [u32], width: usize, height: usize, glow_strength: u32) {
    if glow_strength == 0 || height < 8 {
        return;
    }

    let blend = (glow_strength * 64 / 100).min(64);
    if blend == 0 {
        return;
    }

    let inv = 256 - blend;

    for y in (3..height).step_by(4) {
        let above = y - 1;
        let below = if y + 1 < height { y + 1 } else { y };

        let row = y * width;
        let above_row = above * width;
        let below_row = below * width;

        for x in 0..width {
            let idx = row + x;
            let pixel = unsafe { *buffer.get_unchecked(idx) };
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;

            let pa = unsafe { *buffer.get_unchecked(above_row + x) };
            let pb = unsafe { *buffer.get_unchecked(below_row + x) };

            let avg_r = (((pa >> 16) & 0xFF) + ((pb >> 16) & 0xFF)) >> 1;
            let avg_g = (((pa >> 8) & 0xFF) + ((pb >> 8) & 0xFF)) >> 1;
            let avg_b = ((pa & 0xFF) + (pb & 0xFF)) >> 1;

            let nr = (r * inv + avg_r * blend) >> 8;
            let ng = (g * inv + avg_g * blend) >> 8;
            let nb = (b * inv + avg_b * blend) >> 8;

            unsafe {
                *buffer.get_unchecked_mut(idx) = (nr << 16) | (ng << 8) | nb;
            }
        }
    }
}

// ── CRT filter dispatcher ────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn crt_filter(
    input: &[u32],
    output: &mut Vec<u32>,
    sv_table: &[u8],
    distortion_table: &[(u32, u32)],
    crt_cfg: &CrtConfig,
    mask_table: &[(u16, u16, u16)],
    brightness: i32,
    contrast: i32,
) {
    let blur_side = (25u32 * crt_cfg.blur_amount as u32) / 40;
    let blur_center = 256 - blur_side * 2;
    let use_blur = blur_side > 0;

    if !use_blur && distortion_table_looks_flat(distortion_table) {
        crt_filter_flat(
            input,
            output,
            sv_table,
            distortion_table,
            crt_cfg,
            mask_table,
            brightness,
            contrast,
        );
        return;
    }

    output.resize(SCREEN_W * SCREEN_H, 0);

    let gamma_lut = build_gamma_lut(brightness, contrast);
    let phosphor_lut = build_phosphor_lut(crt_cfg.phosphor_warmth as u32);
    let use_mask = crt_cfg.mask_mode != CrtMaskMode::Off;

    if use_mask && use_blur {
        crt_filter_full(
            input,
            output,
            sv_table,
            distortion_table,
            &phosphor_lut,
            blur_center,
            blur_side,
            mask_table,
            &gamma_lut,
        );
    } else if use_mask {
        crt_filter_masked(
            input,
            output,
            sv_table,
            distortion_table,
            &phosphor_lut,
            mask_table,
            &gamma_lut,
        );
    } else if use_blur {
        crt_filter_blurred(
            input,
            output,
            sv_table,
            distortion_table,
            &phosphor_lut,
            blur_center,
            blur_side,
            &gamma_lut,
        );
    } else {
        crt_filter_basic(
            input,
            output,
            sv_table,
            distortion_table,
            &phosphor_lut,
            &gamma_lut,
        );
    }
}

/// CRT filter fast path for the flat scaler table used when barrel distortion is off.
/// The flat table is separable: source X depends only on destination X, and source Y
/// depends only on destination Y. Keeping that invariant out of the inner loop avoids
/// one geometry-table load and the invalid-pixel branch per output pixel.
#[allow(clippy::too_many_arguments)]
pub fn crt_filter_flat(
    input: &[u32],
    output: &mut Vec<u32>,
    sv_table: &[u8],
    flat_distortion_table: &[(u32, u32)],
    crt_cfg: &CrtConfig,
    mask_table: &[(u16, u16, u16)],
    brightness: i32,
    contrast: i32,
) {
    let blur_side = (25u32 * crt_cfg.blur_amount as u32) / 40;
    if blur_side > 0 {
        crt_filter(
            input,
            output,
            sv_table,
            flat_distortion_table,
            crt_cfg,
            mask_table,
            brightness,
            contrast,
        );
        return;
    }

    output.resize(SCREEN_W * SCREEN_H, 0);

    let gamma_lut = build_gamma_lut(brightness, contrast);
    let phosphor_lut = build_phosphor_lut(crt_cfg.phosphor_warmth as u32);

    if crt_cfg.mask_mode != CrtMaskMode::Off {
        crt_filter_flat_masked(
            input,
            output,
            sv_table,
            flat_distortion_table,
            &phosphor_lut,
            mask_table,
            &gamma_lut,
        );
    } else {
        crt_filter_flat_basic(
            input,
            output,
            sv_table,
            flat_distortion_table,
            &phosphor_lut,
            &gamma_lut,
        );
    }
}

// ── Specialized CRT filter paths ─────────────────────────────────────────────

#[inline(always)]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub fn crt_filter_flat_masked(
    input: &[u32],
    output: &mut Vec<u32>,
    sv_table: &[u8],
    flat_distortion_table: &[(u32, u32)],
    phosphor_lut: &[(u32, u32, u32); 256],
    mask_table: &[(u16, u16, u16)],
    gamma_lut: &[u8; 256],
) {
    debug_assert!(flat_distortion_table.len() >= SCREEN_W * SCREEN_H);
    let x_map = &flat_distortion_table[..SCREEN_W];
    let mut top_rb = [0u32; SCREEN_W];
    let mut bottom_rb = [0u32; SCREEN_W];
    let mut top_g = [0u32; SCREEN_W];
    let mut bottom_g = [0u32; SCREEN_W];
    let mut top_src = usize::MAX;
    let mut bottom_src = usize::MAX;

    for dst_y in 0..SCREEN_H {
        let dst_row = dst_y * SCREEN_W;
        let src_yf = unsafe { flat_distortion_table.get_unchecked(dst_row).1 };
        let src_y0 = (src_yf >> 8) as usize;
        let src_y1 = if src_y0 < 239 { src_y0 + 1 } else { 239 };
        let frac_y = if src_y0 >= 239 { 0 } else { src_yf & 0xFF };

        if top_src != src_y0 {
            if bottom_src == src_y0 {
                let old_top_src = top_src;
                std::mem::swap(&mut top_rb, &mut bottom_rb);
                std::mem::swap(&mut top_g, &mut bottom_g);
                top_src = src_y0;
                bottom_src = old_top_src;
            } else {
                scale_flat_source_row(input, x_map, src_y0, &mut top_rb, &mut top_g);
                top_src = src_y0;
            }
        }

        if bottom_src != src_y1 {
            scale_flat_source_row(input, x_map, src_y1, &mut bottom_rb, &mut bottom_g);
            bottom_src = src_y1;
        }

        let output_row = &mut output[dst_row..dst_row + SCREEN_W];
        for dst_x in 0..SCREEN_W {
            let table_idx = dst_row + dst_x;
            let (mut r, mut g, mut b) = blend_flat_rows_rgb(
                unsafe { *top_rb.get_unchecked(dst_x) },
                unsafe { *bottom_rb.get_unchecked(dst_x) },
                unsafe { *top_g.get_unchecked(dst_x) },
                unsafe { *bottom_g.get_unchecked(dst_x) },
                frac_y,
            );

            let brightness = ((r + g + b) * 85) >> 8;
            let (pr, pg, pb) = unsafe { *phosphor_lut.get_unchecked(brightness as usize) };
            let sv = unsafe { *sv_table.get_unchecked(table_idx) as u32 };
            r = (r * ((pr * sv) >> 8)) >> 8;
            g = (g * ((pg * sv) >> 8)) >> 8;
            b = (b * ((pb * sv) >> 8)) >> 8;

            let (mr, mg, mb) = unsafe { *mask_table.get_unchecked(table_idx) };
            (r, g, b) = apply_mask(r, g, b, mr, mg, mb);

            r = unsafe { *gamma_lut.get_unchecked(r.min(255) as usize) } as u32;
            g = unsafe { *gamma_lut.get_unchecked(g.min(255) as usize) } as u32;
            b = unsafe { *gamma_lut.get_unchecked(b.min(255) as usize) } as u32;
            unsafe {
                *output_row.get_unchecked_mut(dst_x) = pack_rgb_unchecked(r, g, b);
            }
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub fn crt_filter_flat_basic(
    input: &[u32],
    output: &mut Vec<u32>,
    sv_table: &[u8],
    flat_distortion_table: &[(u32, u32)],
    phosphor_lut: &[(u32, u32, u32); 256],
    gamma_lut: &[u8; 256],
) {
    debug_assert!(flat_distortion_table.len() >= SCREEN_W * SCREEN_H);
    let x_map = &flat_distortion_table[..SCREEN_W];
    let mut top_rb = [0u32; SCREEN_W];
    let mut bottom_rb = [0u32; SCREEN_W];
    let mut top_g = [0u32; SCREEN_W];
    let mut bottom_g = [0u32; SCREEN_W];
    let mut top_src = usize::MAX;
    let mut bottom_src = usize::MAX;

    for dst_y in 0..SCREEN_H {
        let dst_row = dst_y * SCREEN_W;
        let src_yf = unsafe { flat_distortion_table.get_unchecked(dst_row).1 };
        let src_y0 = (src_yf >> 8) as usize;
        let src_y1 = if src_y0 < 239 { src_y0 + 1 } else { 239 };
        let frac_y = if src_y0 >= 239 { 0 } else { src_yf & 0xFF };

        if top_src != src_y0 {
            if bottom_src == src_y0 {
                let old_top_src = top_src;
                std::mem::swap(&mut top_rb, &mut bottom_rb);
                std::mem::swap(&mut top_g, &mut bottom_g);
                top_src = src_y0;
                bottom_src = old_top_src;
            } else {
                scale_flat_source_row(input, x_map, src_y0, &mut top_rb, &mut top_g);
                top_src = src_y0;
            }
        }

        if bottom_src != src_y1 {
            scale_flat_source_row(input, x_map, src_y1, &mut bottom_rb, &mut bottom_g);
            bottom_src = src_y1;
        }

        let output_row = &mut output[dst_row..dst_row + SCREEN_W];
        for dst_x in 0..SCREEN_W {
            let table_idx = dst_row + dst_x;
            let (mut r, mut g, mut b) = blend_flat_rows_rgb(
                unsafe { *top_rb.get_unchecked(dst_x) },
                unsafe { *bottom_rb.get_unchecked(dst_x) },
                unsafe { *top_g.get_unchecked(dst_x) },
                unsafe { *bottom_g.get_unchecked(dst_x) },
                frac_y,
            );

            let brightness = ((r + g + b) * 85) >> 8;
            let (pr, pg, pb) = unsafe { *phosphor_lut.get_unchecked(brightness as usize) };
            let sv = unsafe { *sv_table.get_unchecked(table_idx) as u32 };
            r = (r * ((pr * sv) >> 8)) >> 8;
            g = (g * ((pg * sv) >> 8)) >> 8;
            b = (b * ((pb * sv) >> 8)) >> 8;

            r = unsafe { *gamma_lut.get_unchecked(r.min(255) as usize) } as u32;
            g = unsafe { *gamma_lut.get_unchecked(g.min(255) as usize) } as u32;
            b = unsafe { *gamma_lut.get_unchecked(b.min(255) as usize) } as u32;
            unsafe {
                *output_row.get_unchecked_mut(dst_x) = pack_rgb_unchecked(r, g, b);
            }
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub fn crt_filter_full(
    input: &[u32],
    output: &mut Vec<u32>,
    sv_table: &[u8],
    distortion_table: &[(u32, u32)],
    phosphor_lut: &[(u32, u32, u32); 256],
    blur_center: u32,
    blur_side: u32,
    mask_table: &[(u16, u16, u16)],
    gamma_lut: &[u8; 256],
) {
    output
        .chunks_mut(SCREEN_W * PAR_ROWS)
        .enumerate()
        .for_each(|(chunk_idx, chunk_output)| {
            let base_y = chunk_idx * PAR_ROWS;
            let rows_in_chunk = chunk_output.len() / SCREEN_W;

            for local_y in 0..rows_in_chunk {
                let dst_y = base_y + local_y;
                let dst_row = dst_y * SCREEN_W;
                let out_row = local_y * SCREEN_W;

                for dst_x in 0..SCREEN_W {
                    let table_idx = dst_row + dst_x;
                    let (src_xf, src_yf) = unsafe { *distortion_table.get_unchecked(table_idx) };

                    if src_xf == 0xFFFFFFFF {
                        unsafe {
                            *chunk_output.get_unchecked_mut(out_row + dst_x) = 0;
                        }
                        continue;
                    }

                    let src_x0 = (src_xf >> 8) as usize;
                    let src_y0 = (src_yf >> 8) as usize;
                    let src_x1 = (src_x0 + 1).min(255);
                    let src_y1 = (src_y0 + 1).min(239);
                    let frac_x = if src_x0 >= 255 { 0 } else { src_xf & 0xFF };
                    let frac_y = if src_y0 >= 239 { 0 } else { src_yf & 0xFF };

                    let base_offset = src_y0 * 256;
                    let p00 = unsafe { *input.get_unchecked(base_offset + src_x0) };
                    let p10 = unsafe { *input.get_unchecked(base_offset + src_x1) };
                    let p01 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x0) };
                    let p11 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x1) };

                    let (mut r, mut g, mut b) = if frac_x | frac_y == 0 {
                        ((p00 >> 16) & 0xFF, (p00 >> 8) & 0xFF, p00 & 0xFF)
                    } else {
                        blend_bilinear_rgb(p00, p10, p01, p11, frac_x, frac_y)
                    };

                    if src_x0 > 0 && src_x0 < 255 {
                        let left = unsafe { *input.get_unchecked(base_offset + src_x0 - 1) };
                        let right = unsafe { *input.get_unchecked(base_offset + src_x1) };
                        (r, g, b) = apply_blur_3tap(r, g, b, left, right, blur_center, blur_side);
                    }

                    let brightness = ((r + g + b) * 85) >> 8;
                    let (pr, pg, pb) = unsafe { *phosphor_lut.get_unchecked(brightness as usize) };
                    let sv = unsafe { *sv_table.get_unchecked(table_idx) as u32 };
                    r = (r * ((pr * sv) >> 8)) >> 8;
                    g = (g * ((pg * sv) >> 8)) >> 8;
                    b = (b * ((pb * sv) >> 8)) >> 8;

                    let (mr, mg, mb) = unsafe { *mask_table.get_unchecked(table_idx) };
                    (r, g, b) = apply_mask(r, g, b, mr, mg, mb);

                    r = unsafe { *gamma_lut.get_unchecked(r.min(255) as usize) } as u32;
                    g = unsafe { *gamma_lut.get_unchecked(g.min(255) as usize) } as u32;
                    b = unsafe { *gamma_lut.get_unchecked(b.min(255) as usize) } as u32;
                    unsafe {
                        *chunk_output.get_unchecked_mut(out_row + dst_x) =
                            pack_rgb_unchecked(r, g, b);
                    }
                }
            }
        });
}

#[inline(always)]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub fn crt_filter_masked(
    input: &[u32],
    output: &mut Vec<u32>,
    sv_table: &[u8],
    distortion_table: &[(u32, u32)],
    phosphor_lut: &[(u32, u32, u32); 256],
    mask_table: &[(u16, u16, u16)],
    gamma_lut: &[u8; 256],
) {
    output
        .chunks_mut(SCREEN_W * PAR_ROWS)
        .enumerate()
        .for_each(|(chunk_idx, chunk_output)| {
            let base_y = chunk_idx * PAR_ROWS;
            let rows_in_chunk = chunk_output.len() / SCREEN_W;

            for local_y in 0..rows_in_chunk {
                let dst_y = base_y + local_y;
                let dst_row = dst_y * SCREEN_W;
                let out_row = local_y * SCREEN_W;

                for dst_x in 0..SCREEN_W {
                    let table_idx = dst_row + dst_x;
                    let (src_xf, src_yf) = unsafe { *distortion_table.get_unchecked(table_idx) };

                    if src_xf == 0xFFFFFFFF {
                        unsafe {
                            *chunk_output.get_unchecked_mut(out_row + dst_x) = 0;
                        }
                        continue;
                    }

                    let src_x0 = (src_xf >> 8) as usize;
                    let src_y0 = (src_yf >> 8) as usize;
                    let src_x1 = (src_x0 + 1).min(255);
                    let src_y1 = (src_y0 + 1).min(239);
                    let frac_x = if src_x0 >= 255 { 0 } else { src_xf & 0xFF };
                    let frac_y = if src_y0 >= 239 { 0 } else { src_yf & 0xFF };

                    let base_offset = src_y0 * 256;
                    let p00 = unsafe { *input.get_unchecked(base_offset + src_x0) };
                    let p10 = unsafe { *input.get_unchecked(base_offset + src_x1) };
                    let p01 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x0) };
                    let p11 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x1) };

                    let (mut r, mut g, mut b) = if frac_x | frac_y == 0 {
                        ((p00 >> 16) & 0xFF, (p00 >> 8) & 0xFF, p00 & 0xFF)
                    } else {
                        blend_bilinear_rgb(p00, p10, p01, p11, frac_x, frac_y)
                    };

                    let brightness = ((r + g + b) * 85) >> 8;
                    let (pr, pg, pb) = unsafe { *phosphor_lut.get_unchecked(brightness as usize) };
                    let sv = unsafe { *sv_table.get_unchecked(table_idx) as u32 };
                    r = (r * ((pr * sv) >> 8)) >> 8;
                    g = (g * ((pg * sv) >> 8)) >> 8;
                    b = (b * ((pb * sv) >> 8)) >> 8;

                    let (mr, mg, mb) = unsafe { *mask_table.get_unchecked(table_idx) };
                    (r, g, b) = apply_mask(r, g, b, mr, mg, mb);

                    r = unsafe { *gamma_lut.get_unchecked(r.min(255) as usize) } as u32;
                    g = unsafe { *gamma_lut.get_unchecked(g.min(255) as usize) } as u32;
                    b = unsafe { *gamma_lut.get_unchecked(b.min(255) as usize) } as u32;
                    unsafe {
                        *chunk_output.get_unchecked_mut(out_row + dst_x) =
                            pack_rgb_unchecked(r, g, b);
                    }
                }
            }
        });
}

#[inline(always)]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub fn crt_filter_blurred(
    input: &[u32],
    output: &mut Vec<u32>,
    sv_table: &[u8],
    distortion_table: &[(u32, u32)],
    phosphor_lut: &[(u32, u32, u32); 256],
    blur_center: u32,
    blur_side: u32,
    gamma_lut: &[u8; 256],
) {
    output
        .chunks_mut(SCREEN_W * PAR_ROWS)
        .enumerate()
        .for_each(|(chunk_idx, chunk_output)| {
            let base_y = chunk_idx * PAR_ROWS;
            let rows_in_chunk = chunk_output.len() / SCREEN_W;

            for local_y in 0..rows_in_chunk {
                let dst_y = base_y + local_y;
                let dst_row = dst_y * SCREEN_W;
                let out_row = local_y * SCREEN_W;

                for dst_x in 0..SCREEN_W {
                    let table_idx = dst_row + dst_x;
                    let (src_xf, src_yf) = unsafe { *distortion_table.get_unchecked(table_idx) };

                    if src_xf == 0xFFFFFFFF {
                        unsafe {
                            *chunk_output.get_unchecked_mut(out_row + dst_x) = 0;
                        }
                        continue;
                    }

                    let src_x0 = (src_xf >> 8) as usize;
                    let src_y0 = (src_yf >> 8) as usize;
                    let src_x1 = (src_x0 + 1).min(255);
                    let src_y1 = (src_y0 + 1).min(239);
                    let frac_x = if src_x0 >= 255 { 0 } else { src_xf & 0xFF };
                    let frac_y = if src_y0 >= 239 { 0 } else { src_yf & 0xFF };

                    let base_offset = src_y0 * 256;
                    let p00 = unsafe { *input.get_unchecked(base_offset + src_x0) };
                    let p10 = unsafe { *input.get_unchecked(base_offset + src_x1) };
                    let p01 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x0) };
                    let p11 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x1) };

                    let (mut r, mut g, mut b) = if frac_x | frac_y == 0 {
                        ((p00 >> 16) & 0xFF, (p00 >> 8) & 0xFF, p00 & 0xFF)
                    } else {
                        blend_bilinear_rgb(p00, p10, p01, p11, frac_x, frac_y)
                    };

                    if src_x0 > 0 && src_x0 < 255 {
                        let left = unsafe { *input.get_unchecked(base_offset + src_x0 - 1) };
                        let right = unsafe { *input.get_unchecked(base_offset + src_x1) };
                        (r, g, b) = apply_blur_3tap(r, g, b, left, right, blur_center, blur_side);
                    }

                    let brightness = ((r + g + b) * 85) >> 8;
                    let (pr, pg, pb) = unsafe { *phosphor_lut.get_unchecked(brightness as usize) };
                    let sv = unsafe { *sv_table.get_unchecked(table_idx) as u32 };
                    r = (r * ((pr * sv) >> 8)) >> 8;
                    g = (g * ((pg * sv) >> 8)) >> 8;
                    b = (b * ((pb * sv) >> 8)) >> 8;

                    r = unsafe { *gamma_lut.get_unchecked(r.min(255) as usize) } as u32;
                    g = unsafe { *gamma_lut.get_unchecked(g.min(255) as usize) } as u32;
                    b = unsafe { *gamma_lut.get_unchecked(b.min(255) as usize) } as u32;
                    unsafe {
                        *chunk_output.get_unchecked_mut(out_row + dst_x) =
                            pack_rgb_unchecked(r, g, b);
                    }
                }
            }
        });
}

#[inline(always)]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub fn crt_filter_basic(
    input: &[u32],
    output: &mut Vec<u32>,
    sv_table: &[u8],
    distortion_table: &[(u32, u32)],
    phosphor_lut: &[(u32, u32, u32); 256],
    gamma_lut: &[u8; 256],
) {
    output
        .chunks_mut(SCREEN_W * PAR_ROWS)
        .enumerate()
        .for_each(|(chunk_idx, chunk_output)| {
            let base_y = chunk_idx * PAR_ROWS;
            let rows_in_chunk = chunk_output.len() / SCREEN_W;

            for local_y in 0..rows_in_chunk {
                let dst_y = base_y + local_y;
                let dst_row = dst_y * SCREEN_W;
                let out_row = local_y * SCREEN_W;

                for dst_x in 0..SCREEN_W {
                    let table_idx = dst_row + dst_x;
                    let (src_xf, src_yf) = unsafe { *distortion_table.get_unchecked(table_idx) };

                    if src_xf == 0xFFFFFFFF {
                        unsafe {
                            *chunk_output.get_unchecked_mut(out_row + dst_x) = 0;
                        }
                        continue;
                    }

                    let src_x0 = (src_xf >> 8) as usize;
                    let src_y0 = (src_yf >> 8) as usize;
                    let src_x1 = (src_x0 + 1).min(255);
                    let src_y1 = (src_y0 + 1).min(239);
                    let frac_x = if src_x0 >= 255 { 0 } else { src_xf & 0xFF };
                    let frac_y = if src_y0 >= 239 { 0 } else { src_yf & 0xFF };

                    let base_offset = src_y0 * 256;
                    let p00 = unsafe { *input.get_unchecked(base_offset + src_x0) };
                    let p10 = unsafe { *input.get_unchecked(base_offset + src_x1) };
                    let p01 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x0) };
                    let p11 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x1) };

                    let (mut r, mut g, mut b) = if frac_x | frac_y == 0 {
                        ((p00 >> 16) & 0xFF, (p00 >> 8) & 0xFF, p00 & 0xFF)
                    } else {
                        blend_bilinear_rgb(p00, p10, p01, p11, frac_x, frac_y)
                    };

                    let brightness = ((r + g + b) * 85) >> 8;
                    let (pr, pg, pb) = unsafe { *phosphor_lut.get_unchecked(brightness as usize) };
                    let sv = unsafe { *sv_table.get_unchecked(table_idx) as u32 };
                    r = (r * ((pr * sv) >> 8)) >> 8;
                    g = (g * ((pg * sv) >> 8)) >> 8;
                    b = (b * ((pb * sv) >> 8)) >> 8;

                    r = unsafe { *gamma_lut.get_unchecked(r.min(255) as usize) } as u32;
                    g = unsafe { *gamma_lut.get_unchecked(g.min(255) as usize) } as u32;
                    b = unsafe { *gamma_lut.get_unchecked(b.min(255) as usize) } as u32;
                    unsafe {
                        *chunk_output.get_unchecked_mut(out_row + dst_x) =
                            pack_rgb_unchecked(r, g, b);
                    }
                }
            }
        });
}

// ── Combined gamma + brightness + contrast pass ──────────────────────────────

#[allow(dead_code)]
#[inline]
pub fn apply_gamma_brightness_contrast(
    buffer: &mut [u32],
    len: usize,
    brightness: i32,
    contrast: i32,
) {
    if brightness == 0 && contrast == 0 {
        return;
    }
    let mut lut = [0u8; 256];
    let con_scale = 256 + (contrast * 256 / 50);

    for i in 0..256 {
        let gamma_val = GAMMA_TABLE[i] as i32;
        let val = (((gamma_val - 128) * con_scale) >> 8) + 128 + brightness * 255 / 50;
        lut[i] = val.clamp(0, 255) as u8;
    }

    for i in 0..len {
        let pixel = unsafe { *buffer.get_unchecked(i) };
        let r = lut[((pixel >> 16) & 0xFF) as usize] as u32;
        let g = lut[((pixel >> 8) & 0xFF) as usize] as u32;
        let b = lut[(pixel & 0xFF) as usize] as u32;
        unsafe {
            *buffer.get_unchecked_mut(i) = (r << 16) | (g << 8) | b;
        }
    }
}

// ── Simple scaler ────────────────────────────────────────────────────────────

pub fn scale_simple(input: &[u32], output: &mut Vec<u32>) {
    output.resize(SCREEN_W * SCREEN_H, 0);
    for y in 0..SCREEN_H {
        let src_y = y * 240 / SCREEN_H;
        for x in 0..SCREEN_W {
            let src_x = x * 256 / SCREEN_W;
            output[y * SCREEN_W + x] = input[src_y * 256 + src_x.min(255)];
        }
    }
}

// ── Glass inner loop ─────────────────────────────────────────────────────────

#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn glass_inner_loop(
    buffer: &mut [u32],
    ghost_source: &[u32],
    glare_table: &[u8],
    thickness_table: &[u16],
    ghost_alpha_table: &[u8],
    window_width: usize,
    intensity_factor: u32,
    tint_strength: u32,
    do_ghost: bool,
    corner_x_max: usize,
    corner_y_max: usize,
    ghost_shift_x: usize,
    ghost_shift_y: usize,
    ghost_h: usize,
    ghost_w: usize,
    ghost_stride: usize,
) {
    const CORNER_R: usize = 10;
    const CORNER_R_SQ: usize = CORNER_R * CORNER_R;

    let buf_start = SCREEN_Y * window_width;
    let buf_end = (SCREEN_Y + SCREEN_H) * window_width;
    buffer[buf_start..buf_end]
        .chunks_mut(window_width * PAR_ROWS)
        .enumerate()
        .for_each(|(chunk_idx, chunk_buf)| {
            let base_y = chunk_idx * PAR_ROWS;
            let rows_in_chunk = chunk_buf.len() / window_width;

            for local_y in 0..rows_in_chunk {
                let y = base_y + local_y;
                let glare_row = y * SCREEN_W;

                let in_corner_y_top = y < CORNER_R;
                let in_corner_y_bottom = y >= corner_y_max;

                let ghost_src_row = if do_ghost && y < ghost_h {
                    Some((y + ghost_shift_y) * ghost_stride)
                } else {
                    None
                };

                for x in 0..SCREEN_W {
                    if (in_corner_y_top || in_corner_y_bottom)
                        && (x < CORNER_R || x >= corner_x_max)
                    {
                        let (cx, cy) = if x < CORNER_R {
                            if in_corner_y_top {
                                (CORNER_R, CORNER_R)
                            } else {
                                (CORNER_R, SCREEN_H - 1 - CORNER_R)
                            }
                        } else if in_corner_y_top {
                            (SCREEN_W - 1 - CORNER_R, CORNER_R)
                        } else {
                            (SCREEN_W - 1 - CORNER_R, SCREEN_H - 1 - CORNER_R)
                        };
                        if sq_dist(x, y, cx, cy) > CORNER_R_SQ {
                            continue;
                        }
                    }

                    let glare_idx = glare_row + x;
                    let glare_base = unsafe { *glare_table.get_unchecked(glare_idx) as u32 };

                    let buf_idx = local_y * window_width + SCREEN_X + x;
                    let pixel = unsafe { *chunk_buf.get_unchecked(buf_idx) };
                    let mut r = (pixel >> 16) & 0xFF;
                    let mut g = (pixel >> 8) & 0xFF;
                    let mut b = pixel & 0xFF;

                    let grey = ((r + g + b) * 171) >> 9;

                    if tint_strength > 1 {
                        let thickness =
                            256 + unsafe { *thickness_table.get_unchecked(glare_idx) } as u32;
                        let tint = tint_strength * thickness / 256;

                        r = r + ((grey.saturating_sub(r) * tint * 205) >> 13);
                        g = g + ((grey.saturating_sub(g) * tint * 182) >> 13);
                        b = b.saturating_sub((tint * 171) >> 9);
                    }

                    if glare_base > 0 {
                        let glare =
                            (glare_base * intensity_factor * (200_u32.saturating_sub(grey)) * 29)
                                >> 19;

                        r = (r + glare + ((glare * 171) >> 11)).min(255);
                        g = (g + glare).min(255);
                        b = (b + glare.saturating_sub((glare * 17) >> 8)).min(255);
                    }

                    if let Some(src_row) = ghost_src_row {
                        if x < ghost_w {
                            let local_alpha =
                                unsafe { *ghost_alpha_table.get_unchecked(glare_idx) } as u32;
                            if local_alpha > 0 {
                                let src_idx = src_row + x + ghost_shift_x;
                                let ghost_pixel = unsafe { *ghost_source.get_unchecked(src_idx) };
                                let inv_alpha = 256 - local_alpha;
                                r = ((r * inv_alpha + ((ghost_pixel >> 16) & 0xFF) * local_alpha)
                                    >> 8)
                                    .min(255);
                                g = ((g * inv_alpha + ((ghost_pixel >> 8) & 0xFF) * local_alpha)
                                    >> 8)
                                    .min(255);
                                b = ((b * inv_alpha + (ghost_pixel & 0xFF) * local_alpha) >> 8)
                                    .min(255);
                            }
                        }
                    }

                    unsafe {
                        *chunk_buf.get_unchecked_mut(buf_idx) = (r << 16) | (g << 8) | b;
                    }
                }
            }
        });
}
