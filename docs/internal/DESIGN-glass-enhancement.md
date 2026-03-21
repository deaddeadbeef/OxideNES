# CRT Glass Enhancement — Technical Design

## PROBLEM
Current glass effects (chromatic aberration + flat Gaussian specular highlights) look artificial.
Real CRT glass is a thick curved optical element that tints the image, creates internal ghost
reflections, and produces Fresnel-modulated reflections that follow the glass curvature. The
existing `build_glare_table()` uses arbitrary Gaussian blobs that don't correspond to any
physical reflection model.

---

## EXISTING SYSTEM SUMMARY

```
Pipeline:  crt_filter → CA → composite_screen → screen_glare
Buffers:   crt_buffer  [820×769 u32]
           ca_temp     [820×769 u32]  (snapshot for CA source reads)
           composite_buffer [1200×1060 u32]
Constants: SCREEN_W=820, SCREEN_H=769, SCREEN_X=190, SCREEN_Y=50
Control:   glass_intensity: u8 (0-100, default 60)
Pixel fmt: 0x00RRGGBB u32
```

### What exists and where
| Function | Line | Table type | Size | Rebuilt? |
|----------|------|-----------|------|----------|
| `build_glare_table()` | 5454 | `Vec<u8>` | 630K × 1B = 631KB | Never (static) |
| `build_ca_table()` | 5569 | `CaTable { shifts: Vec<(i16,i16)> }` | 630K × 4B = 2.5MB | On intensity change |
| `apply_screen_glare()` | 5496 | — | — | — |
| `apply_chromatic_aberration()` | 5599 | — | — | — |

---

## APPROACHES

### Approach A: Enhanced Table Values Only (Minimal Change)
Replace `build_glare_table()` internals with curved-surface-normal math and Fresnel; keep
everything else the same. Add glass tint as a post-process.

- **Pros**: Smallest diff, no new pipeline stages, same memory footprint
- **Cons**: No ghost reflection (the most distinctive CRT look). Glare table is still a single
  `u8` per pixel — can't represent colored environment reflections.

### Approach B: Full Physical Glass Model (3 New Stages)
Add glass tint, ghost reflection, and completely rework glare with curved surface normals,
Fresnel reflections, and environment mapping. Three new pipeline stages.

- **Pros**: Most realistic result, covers all 6 design requirements
- **Cons**: 3 new build functions + 3 new apply functions, larger code footprint,
  ~3 new tables (~3MB total). Pipeline has more stages.

### Approach C: Targeted Additions (2 New Stages, 1 Enhanced) ← RECOMMENDED
Add glass tint (trivial LUT) and ghost reflection (new table + apply). Rework `build_glare_table`
with curved-surface Fresnel model. Fold environment reflection into the enhanced glare table
(keeps it as `Vec<u8>`, no format change). Reuse existing `ca_temp` buffer for ghost source.

- **Pros**: Highest visual impact (ghost + Fresnel are the two most "CRT-looking" effects).
  Glare table stays same format (`Vec<u8>`) so `apply_screen_glare` needs only minor tweaks.
  Glass tint is a 256-byte LUT — negligible memory/compute. Reuses `ca_temp` so no new buffer
  allocation. Only 2 new functions in the pipeline.
- **Cons**: Environment reflection is monochrome (folded into u8 glare table rather than RGB).
  This is acceptable because real CRT environment reflections are dim and nearly achromatic.

---

## RECOMMENDATION: Approach C

Best visual-impact-to-complexity ratio. The ghost reflection and Fresnel model are the two
effects that most strongly say "this is thick curved glass" to the viewer. Glass tint adds
realism for almost zero cost. Folding environment reflection into the glare table avoids a
new table format while still capturing the effect.

---

## IMPLEMENTATION PLAN

### New Pipeline Order

```
crt_filter
  → chromatic_aberration    (existing, unchanged)
  → apply_ghost_reflection  (NEW — internal glass reflection)
  → apply_glass_tint        (NEW — glass absorption/contrast reduction)
  → composite_screen        (existing, unchanged)
  → apply_screen_glare      (ENHANCED — Fresnel + curved specular + environment)
```

### Step 0: Constants and Shared Geometry

Add these constants near the existing CRT constants (around line 26-34):

```rust
// Glass optical model constants
const GLASS_CURVATURE_R: f64 = 2.5;  // Radius of curvature as multiple of screen diagonal
                                      // 2.5 = gentle CRT curve, 1.5 = deep curve
```

Pre-compute these derived values in `build_glass_reflection_table()`:
```
screen_diag = sqrt(SCREEN_W² + SCREEN_H²)    // ≈ 1123 pixels
R = screen_diag * GLASS_CURVATURE_R           // ≈ 2808 pixels
cx = SCREEN_W / 2.0                           // 410.0
cy = SCREEN_H / 2.0                           // 384.5
```

Surface normal per pixel (used by both glare and ghost):
```
nx = (x - cx) / R
ny = (y - cy) / R
nz = sqrt(max(0.0, 1.0 - nx² - ny²))
```

At center: `(nx, ny, nz) ≈ (0, 0, 1)` — looking straight through.
At edge:   `nz` drops, meaning more oblique viewing angle.

---

### Step 1: NEW — `build_glass_tint_lut(glass_intensity: u8) -> [u8; 256]`

**What**: CRT glass absorbs light. This reduces contrast and shifts colors toward a dark
neutral grey. Modeled as a simple per-channel affine transform.

**Algorithm**:
```
tint_strength = (glass_intensity as u32 * 18) / 100
// At default 60: strength = 10 (out of 256 scale)
// At 100: strength = 18

For each input value v in 0..256:
    // Linear blend toward grey_point (absorptive glass model)
    // grey_point = 12 represents the slight ambient light trapped in glass
    output[v] = ((v * (256 - tint_strength) + 12 * tint_strength) >> 8) as u8
```

At default intensity (60), this maps:
- Black (0) → 0 (stays black, glass can't brighten darkness)
  - Actually: `(0 * 246 + 12 * 10) / 256 = 120/256 = 0` — rounds to 0. Good.
- Pure white (255) → `(255 * 246 + 12 * 10) / 256 = 62850/256 + 120/256 = 245`. So 255→245.
- Mid grey (128) → `(128 * 246 + 12 * 10) / 256 = 31488/256 + 120/256 = 123`. So 128→123.

This is a ~4% contrast reduction at default settings. Subtle but perceptible.

**Memory**: 256 bytes. **Rebuild**: When `glass_intensity` changes (trivial cost).

**Location in code**: Define just above `build_ca_table()` (around line 5560).

---

### Step 2: NEW — `build_ghost_table() -> GhostTable`

**What**: Thick CRT glass creates a faint secondary image from light reflecting off the
inner surface of the front glass, bouncing off the phosphor coating, and passing through the
glass again. This ghost is shifted radially toward center and strongest at screen edges.

**Struct**:
```rust
struct GhostTable {
    /// Per pixel: (shift_x, shift_y) in pixels for ghost source sampling
    shifts: Vec<(i8, i8)>,
    /// Per pixel: ghost opacity (0-255 scale, pre-Fresnel, pre-intensity)
    /// Actual applied opacity = table_opacity * glass_intensity / (100 * 256)
    opacity: Vec<u8>,
}
```

**Algorithm** (build-time, float math):
```
// Ghost shift model: internal reflection bounces light inward
// Shift magnitude proportional to distance from center and glass curvature
max_ghost_shift = 4.0  // pixels at extreme edge, at full intensity
cx = SCREEN_W / 2.0
cy = SCREEN_H / 2.0

For each pixel (x, y):
    dx = (x - cx) / cx     // normalized -1..1
    dy = (y - cy) / cy
    dist = sqrt(dx² + dy²).min(1.0)

    // Smooth onset: ghost appears gradually from center outward
    // Using smoothstep from 0.25 to 0.95
    t = clamp((dist - 0.25) / 0.70, 0.0, 1.0)
    edge_factor = t * t * (3.0 - 2.0 * t)    // smoothstep

    // Shift direction: toward center (negative of radial direction)
    if dist > 0.01:
        shift_x = round(-dx / dist * max_ghost_shift * edge_factor) as i8
        shift_y = round(-dy / dist * max_ghost_shift * edge_factor) as i8
    else:
        shift_x = 0
        shift_y = 0

    // Opacity: Fresnel-like falloff
    // Ghost opacity maxes at ~6% (15/256) at extreme edges
    // Use dist² for natural falloff
    opacity = round(15.0 * edge_factor * edge_factor) as u8
```

At screen center: `edge_factor ≈ 0`, opacity = 0, shift = (0,0) → no ghost.
At dist = 0.6: `t = 0.5, edge_factor ≈ 0.5`, opacity ≈ 4, shift ≈ 2px.
At screen edge (dist = 1.0): `edge_factor ≈ 1.0`, opacity = 15, shift ≈ 4px.

**Memory**: 630K × 3B ≈ 1.9MB (but `shifts` and `opacity` are separate vecs for
cache-friendly access in the apply loop).

Alternative: store as `Vec<(i8, i8, u8)>` packed — 1.9MB. Same perf since we access all
three fields per pixel anyway.

**Rebuild**: Never (static geometry). glass_intensity applied at runtime in apply function.

**Location in code**: Define after `build_ca_table` (around line 5595), before `apply_chromatic_aberration`.

---

### Step 3: NEW — `apply_ghost_reflection(buffer: &mut [u32], source: &[u32], ghost_table: &GhostTable, glass_intensity: u8)`

**What**: Additively blend a shifted, dimmed copy of the image into the output buffer.

**Source buffer**: `ca_temp` — this already contains the pre-CA snapshot of `crt_buffer`
(line 3623). The ghost physically originates from the same image before chromatic dispersion,
so using the pre-CA source is physically correct AND avoids any new allocation.

**Algorithm** (runtime, integer-only):
```rust
if glass_intensity == 0 { return; }
let gi = glass_intensity as u32;

for idx in 0..SCREEN_W * SCREEN_H:
    let opacity_base = ghost_table.opacity[idx] as u32;  // 0-15
    if opacity_base == 0 { continue; }

    let (sx, sy) = ghost_table.shifts[idx];
    if sx == 0 && sy == 0 { continue; }

    let y = idx / SCREEN_W;
    let x = idx % SCREEN_W;
    let src_x = (x as i32 + sx as i32).clamp(0, SCREEN_W as i32 - 1) as usize;
    let src_y = (y as i32 + sy as i32).clamp(0, SCREEN_H as i32 - 1) as usize;

    // Effective opacity: table_value * glass_intensity / 100
    // Then divide by 256 for the blend
    // Combined: ghost_contrib = channel * opacity_base * gi / 25600
    // Simplify: ghost_contrib = channel * (opacity_base * gi) / 25600
    let alpha = opacity_base * gi;  // max: 15 * 100 = 1500

    let ghost_px = source[src_y * SCREEN_W + src_x];
    let gr = (ghost_px >> 16) & 0xFF;
    let gg = (ghost_px >> 8) & 0xFF;
    let gb = ghost_px & 0xFF;

    let pixel = buffer[idx];
    let r = ((pixel >> 16) & 0xFF + gr * alpha / 25600).min(255);
    let g = ((pixel >> 8) & 0xFF + gg * alpha / 25600).min(255);
    let b = (pixel & 0xFF + gb * alpha / 25600).min(255);

    buffer[idx] = (r << 16) | (g << 8) | b;
```

**Performance note**: The `opacity == 0` early exit skips ~50% of pixels (the center region).
The remaining ~315K pixels each do: 1 table read + 1 source read + 1 buffer read + 3 multiply
+ 3 add + 3 min + 1 write ≈ 15 ops. Total: ~4.7M ops → **~1.6ms** on a single core at 3GHz.

**IMPORTANT — division optimization**: `/ 25600` is expensive. Pre-compute a shift-based
approximation:
```rust
// 25600 ≈ 256 * 100. So: channel * opacity_base * gi / (256 * 100)
// Rearrange: (channel * opacity_base * gi) >> 8 / 100
// Or use: (channel * alpha + 12800) / 25600  for rounding
// Better: precompute alpha_shift = (opacity_base * gi * 256) / 25600
//       = (opacity_base * gi) * 256 / 25600
//       = opacity_base * gi / 100
// Then: contrib = (channel * alpha_shift) >> 8
// This replaces division with shift.
```

Recommended inner loop:
```rust
let alpha_shift = (opacity_base * gi) / 100;  // 0..15 range (max 15*100/100=15)
let r = ((pixel >> 16) & 0xFF) + ((gr * alpha_shift) >> 8);
// etc.
```

This gives max ghost contribution per channel: `255 * 15 / 256 ≈ 15` brightness levels.
At default intensity (60): max contribution ≈ `255 * 9 / 256 ≈ 9`.
Subtle but visible on dark backgrounds — exactly right for internal glass reflection.

---

### Step 4: NEW — `apply_glass_tint(buffer: &mut [u32], tint_lut: &[u8; 256])`

**What**: Apply the pre-computed glass tint LUT to every pixel in the screen buffer.

**Algorithm** (runtime, LUT-only — zero multiplications):
```rust
for pixel in buffer[..SCREEN_W * SCREEN_H].iter_mut() {
    let p = *pixel;
    let r = tint_lut[((p >> 16) & 0xFF) as usize] as u32;
    let g = tint_lut[((p >> 8) & 0xFF) as usize] as u32;
    let b = tint_lut[(p & 0xFF) as usize] as u32;
    *pixel = (r << 16) | (g << 8) | b;
}
```

**Performance**: 3 LUT lookups + 3 shifts + 2 ORs per pixel = 8 ops × 630K = 5M ops.
**~1.7ms** single-core. Could be combined with ghost apply loop to save one pass, but
keeping them separate is cleaner and the cost is acceptable.

**Location in code**: Define near `apply_chromatic_aberration` (around line 5620).

---

### Step 5: ENHANCE — `build_glass_reflection_table() -> Vec<u8>` (replaces `build_glare_table`)

**What**: Complete rewrite of the glare computation using a curved-surface optical model.
Same output format (`Vec<u8>`, same size), so `apply_screen_glare` needs only minor changes.

**Algorithm** (build-time, float math):

```rust
fn build_glass_reflection_table() -> Vec<u8> {
    let mut table = vec![0u8; SCREEN_W * SCREEN_H];

    let cx = SCREEN_W as f64 / 2.0;
    let cy = SCREEN_H as f64 / 2.0;
    let diag = ((SCREEN_W * SCREEN_W + SCREEN_H * SCREEN_H) as f64).sqrt();
    let R = diag * GLASS_CURVATURE_R;   // sphere radius ≈ 2808

    // === Light sources (normalized directions pointing toward screen) ===
    // Primary: ceiling fluorescent, slightly left and above
    let light1 = normalize((-0.25, -0.40, 0.88));  // upper-left
    let light1_power = 55.0;   // specular intensity
    let light1_exp = 28.0;     // shininess (higher = tighter highlight)

    // Secondary: window/ambient from right side
    let light2 = normalize((0.45, -0.15, 0.88));   // upper-right
    let light2_power = 25.0;
    let light2_exp = 16.0;     // broader, dimmer

    // Tertiary: dim fill light from below (desk lamp bounce)
    let light3 = normalize((0.10, 0.50, 0.86));    // lower
    let light3_power = 12.0;
    let light3_exp = 12.0;

    // View direction (straight at screen)
    let view = (0.0, 0.0, 1.0);

    for y in 0..SCREEN_H {
        for x in 0..SCREEN_W {
            // --- Surface normal from spherical glass model ---
            let nx = (x as f64 - cx) / R;
            let ny = (y as f64 - cy) / R;
            let nz_sq = 1.0 - nx * nx - ny * ny;
            let nz = if nz_sq > 0.0 { nz_sq.sqrt() } else { 0.001 };

            // --- Fresnel reflectance (Schlick's approximation) ---
            // F = F0 + (1 - F0)(1 - cos θ)^5
            // cos θ = dot(normal, view) = nz (since view = (0,0,1))
            let f0: f64 = 0.04;   // glass at normal incidence
            let one_minus_cos = 1.0 - nz;
            let omc2 = one_minus_cos * one_minus_cos;
            let fresnel = f0 + (1.0 - f0) * omc2 * omc2 * one_minus_cos; // (1-cos)^5

            // --- Specular highlights (Blinn-Phong on curved surface) ---
            // For each light: half-vector H = normalize(L + V)
            // spec = max(0, dot(N, H))^power

            let mut total_spec: f64 = 0.0;

            // Light 1
            {
                let (hx, hy, hz) = normalize_tuple(
                    light1.0 + view.0,
                    light1.1 + view.1,
                    light1.2 + view.2,
                );
                let ndoth = (nx * hx + ny * hy + nz * hz).max(0.0);
                total_spec += ndoth.powf(light1_exp) * light1_power;
            }

            // Light 2
            {
                let (hx, hy, hz) = normalize_tuple(
                    light2.0 + view.0,
                    light2.1 + view.1,
                    light2.2 + view.2,
                );
                let ndoth = (nx * hx + ny * hy + nz * hz).max(0.0);
                total_spec += ndoth.powf(light2_exp) * light2_power;
            }

            // Light 3
            {
                let (hx, hy, hz) = normalize_tuple(
                    light3.0 + view.0,
                    light3.1 + view.1,
                    light3.2 + view.2,
                );
                let ndoth = (nx * hx + ny * hy + nz * hz).max(0.0);
                total_spec += ndoth.powf(light3_exp) * light3_power;
            }

            // --- Environment ambient reflection ---
            // Faint uniform room reflection, modulated by Fresnel
            // Simulates diffuse room light bouncing off the glass surface
            let env_ambient = 8.0;  // base ambient reflection level

            // --- Wide diagonal reflection band (kept from original, improved) ---
            // This represents an elongated light source (fluorescent tube) reflection
            // stretched along the glass curvature
            let fx = (x as f64 / SCREEN_W as f64) * 2.0 - 1.0;
            let fy = (y as f64 / SCREEN_H as f64) * 2.0 - 1.0;
            let diag_axis = (fx + fy) / std::f64::consts::SQRT_2;
            let band = (-diag_axis * diag_axis * 6.0).exp() * 12.0;

            // --- Combine all layers, modulated by Fresnel ---
            // Specular: full Fresnel modulation
            // Band: partial Fresnel (it's already a reflection)
            // Ambient: full Fresnel modulation
            let fresnel_scale = fresnel / 0.04;  // normalize so center ≈ 1.0
            let combined = total_spec * fresnel_scale.max(0.5)
                         + band * fresnel_scale.max(0.3)
                         + env_ambient * fresnel_scale;

            // --- Corner fade (soft edge near border) ---
            let edge_dist = fx.abs().max(fy.abs());
            let border_fade = if edge_dist > 0.97 {
                ((1.0 - edge_dist) / 0.03).max(0.0)
            } else {
                1.0
            };

            let value = (combined * border_fade).max(0.0).min(80.0) as u8;

            // Zero out 4px border (glass-bezel junction, same as original)
            let in_border = x < 4 || x >= SCREEN_W - 4 || y < 4 || y >= SCREEN_H - 4;
            table[y * SCREEN_W + x] = if in_border { 0 } else { value };
        }
    }
    table
}
```

**Key differences from current `build_glare_table()`**:

| Aspect | Current | Enhanced |
|--------|---------|----------|
| Specular model | Arbitrary Gaussian blobs at fixed positions | Blinn-Phong on curved surface normals |
| Fresnel | Simple `(edge - 0.7)²` threshold | Schlick's `F0 + (1-F0)(1-cos θ)^5` |
| Light sources | 2 (upper-left, lower-right) | 3 (upper-left, upper-right, lower fill) |
| Light placement | Hardcoded screen-space positions | 3D direction vectors reflected by surface normals |
| Environment | None | 8.0 base ambient, Fresnel-modulated |
| Max table value | 50 | 80 (compensated in apply by scaling) |
| Diagonal band | `exp(-d²*4) * 18` | `exp(-d²*6) * 12` (tighter, Fresnel-modulated) |

**Why 3 lights**: Real rooms have multiple light sources. The third fill light from below
(desk lamp / monitor reflection) prevents the lower screen from being too dark and adds
depth to the curved-glass illusion.

**Memory**: Same as current — `Vec<u8>`, 630KB.

**Helper function** needed (define as inline):
```rust
#[inline]
fn normalize_tuple(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let len = (x * x + y * y + z * z).sqrt();
    if len > 0.0001 { (x / len, y / len, z / len) } else { (0.0, 0.0, 1.0) }
}
```

---

### Step 6: ENHANCE — `apply_screen_glare()` (minor modification)

**Changes** to existing function at line 5496:

1. **Increase denominator** from 20000 to 25000 to compensate for the higher max table
   values (80 vs 50). This keeps the visual intensity in the same range at default settings.

2. **Add slight color variation**: Real reflections have a faint cool tint from glass.
   Instead of adding the same glare value to R, G, B, add slightly more to the blue channel.

**Modified inner loop** (changes marked with `// CHANGED`):
```rust
let glare = (glare_base * intensity_factor * (200_u32.saturating_sub(brightness))) / 25000; // CHANGED: was 20000

// Slight cool tint on reflections (glass has higher blue reflectance)     // NEW
let glare_r = glare;                                                        // NEW
let glare_b = glare + (glare >> 3);  // +12.5% blue in reflections         // NEW

let r = (r + glare_r).min(255);       // CHANGED: was glare
let g = (g + glare).min(255);
let b = (b + glare_b).min(255);       // CHANGED: was glare
```

The blue shift is `glare >> 3` — at max glare of ~25, this adds 3 extra blue. Very subtle
but subconsciously reads as "glass surface" rather than "white overlay."

---

### Step 7: ENHANCE — `build_ca_table()` (minor improvement)

**Change**: Extend the CA zone slightly inward and add curvature-aware distortion.

Current: outer 15% ring (`edge_factor > 0.85`), uniform radial.
Enhanced: outer 25% ring (`edge_factor > 0.75`), with the inner portion very subtle.

**Modified lines in build_ca_table** (line ~5583):
```rust
// OLD:
// if edge_factor > 0.85 {
//     let strength = ((edge_factor - 0.85) / 0.15).min(1.0) * intensity_factor;

// NEW: Wider zone with cubic ramp (gentle onset, strong at edge)
if edge_factor > 0.75 {
    let t = ((edge_factor - 0.75) / 0.25).min(1.0);
    let strength = t * t * t * intensity_factor;  // cubic ramp: very subtle at 0.75, full at 1.0
```

This distributes the CA more naturally. At `edge_factor = 0.85` (the old threshold), the
cubic `t = (0.1/0.25)³ = 0.064` — barely 6% of full strength. At `edge_factor = 0.95`:
`t = (0.2/0.25)³ = 0.512` — 51%. The CA is now visible over a wider area but stays subtle
until the very edge. This matches real thick glass CA more accurately.

---

### Step 8: Integration — Modified Render Pipeline

**At initialization** (around line 1875-1881), add new table builds:

```rust
let glare_table = build_glass_reflection_table();  // RENAMED from build_glare_table()
let ghost_table = build_ghost_table();              // NEW
let mut tint_lut = build_glass_tint_lut(glass_intensity);  // NEW
```

**Where glass_intensity changes** (lines ~2161 and ~2270), add tint rebuild:

```rust
ca_table = build_ca_table(SCREEN_W, SCREEN_H, glass_intensity);
tint_lut = build_glass_tint_lut(glass_intensity);  // NEW — 256 iterations, trivial
```

**In the render loop** (lines 3618-3635), insert new stages:

```rust
if crt_enabled {
    crt_filter(&bus.ppu.frame_data, &mut crt_buffer, &vignette_table, dt, &config.crt_config, &mask_table);
    if glass_intensity > 0 {
        ca_temp.copy_from_slice(&crt_buffer[..SCREEN_W * SCREEN_H]);
        apply_chromatic_aberration(&mut crt_buffer, &ca_temp, &ca_table, SCREEN_W, SCREEN_H);
        apply_ghost_reflection(&mut crt_buffer, &ca_temp, &ghost_table, glass_intensity); // NEW
        apply_glass_tint(&mut crt_buffer, &tint_lut);                                     // NEW
    }
} else {
    scale_simple(&bus.ppu.frame_data, &mut crt_buffer);
}

composite_screen(&tv_frame_bg, &crt_buffer, &mut composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT);

if crt_enabled {
    apply_screen_glare(&mut composite_buffer, &glare_table, WINDOW_WIDTH, glass_intensity); // enhanced internals
}
```

**IMPORTANT**: The same changes must be applied to ALL THREE render sites:
1. **Game render loop** — lines 3618-3635
2. **Pause menu render** — lines 2809-2824
3. **Menu/ROM selector render** — lines 2706-2711 (this one doesn't apply CA/glass currently;
   consider whether to add it for consistency)

---

## SUMMARY OF ALL CHANGES

| # | Type | Function | Description |
|---|------|----------|-------------|
| 1 | **NEW** | `build_glass_tint_lut()` | 256-byte LUT for glass absorption. Rebuild on intensity change. |
| 2 | **NEW** | `GhostTable` struct | Holds `(i8, i8, u8)` per pixel: shift + opacity. |
| 3 | **NEW** | `build_ghost_table()` | Builds ghost table using smoothstep + radial shift. Static. |
| 4 | **NEW** | `apply_ghost_reflection()` | Additively blends shifted ghost from `ca_temp`. |
| 5 | **NEW** | `apply_glass_tint()` | Applies tint LUT to crt_buffer. Zero multiplies. |
| 6 | **REPLACE** | `build_glare_table()` → `build_glass_reflection_table()` | Blinn-Phong + Fresnel + 3 lights + environment. Same `Vec<u8>` format. |
| 7 | **ENHANCE** | `apply_screen_glare()` | Adjust denominator (20000→25000), add cool blue tint to reflections. |
| 8 | **ENHANCE** | `build_ca_table()` | Wider zone (0.85→0.75 threshold), cubic ramp instead of linear. |
| 9 | **NEW** | `normalize_tuple()` | Inline helper for vector normalization. |
| 10 | **MODIFY** | Render pipeline (3 sites) | Insert ghost + tint stages after CA, before composite. |

---

## PERFORMANCE BUDGET

| Stage | Pixels touched | Ops/pixel | Estimated time |
|-------|---------------|-----------|----------------|
| glass_tint (LUT) | 630K (all) | 8 | ~1.7ms |
| ghost_reflection | ~315K (50% skipped) | 15 | ~1.6ms |
| enhanced glare (same as before) | ~400K (non-zero) | 12 | ~1.6ms |
| **Total added per frame** | | | **~3.3ms** |

Frame budget at 60fps: 16.67ms. Existing CRT pipeline uses ~8-10ms. New total: ~13ms.
**3.5ms headroom remaining.** Safe.

**Build-time costs** (one-time at init):
- `build_glass_reflection_table()`: ~630K pixels × trig + pow = ~100ms (same as current)
- `build_ghost_table()`: ~630K pixels × sqrt + smoothstep = ~50ms
- `build_glass_tint_lut()`: 256 iterations = <0.01ms

---

## SUGGESTED DEFAULT VALUES

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| `glass_intensity` | 60 (unchanged) | Existing default works well for new effects |
| `GLASS_CURVATURE_R` | 2.5 | Gentle curve matching typical 20" CRT |
| Glass tint `grey_point` | 12 | Very dark — CRT glass mostly absorbs, doesn't reflect to grey |
| Glass tint max `strength` | 18/256 at intensity=100 | ~7% contrast reduction max. Noticeable but not objectionable |
| Ghost `max_shift` | 4 px | Visible but not distracting. At 820px width, 4px ≈ 0.5% |
| Ghost max `opacity` | 15/256 ≈ 6% | Ghosting should be subliminal, not overt |
| Ghost `smoothstep` range | 0.25 → 0.95 | Ghost starts in middle third, strongest at edges |
| Specular shininess | 28, 16, 12 | Mix of tight and broad highlights |
| Specular intensities | 55, 25, 12 | Clear hierarchy: primary > secondary > fill |
| Fresnel F0 | 0.04 | Standard glass index of refraction (n ≈ 1.5) |
| Env ambient | 8.0 | Very subtle room fill, only visible via Fresnel at edges |
| Glare max value | 80 | Up from 50; compensated by 20000→25000 in apply |
| CA zone threshold | 0.75 | Wider than 0.85 but cubic ramp keeps center clean |

---

## RISKS AND MITIGATIONS

1. **Ghost reflection on bright content looks wrong**: When the screen is all white, the ghost
   adds brightness that could clip. **Mitigation**: Ghost uses additive blend with `.min(255)`.
   Max contribution is 9 brightness levels at default intensity — negligible on bright content.

2. **Three render sites must be updated consistently**: Lines 3618, 2809, and 2706. Missing one
   causes visual inconsistency between game/menu/pause. **Mitigation**: Consider extracting a
   `render_with_glass_effects()` helper that encapsulates the CA→ghost→tint chain.

3. **Tint LUT at intensity 0 must be identity**: If `glass_intensity == 0`, the LUT must map
   every value to itself (tint_strength = 0). **Mitigation**: Formula naturally produces
   identity when `tint_strength = 0` since `(v * 256 + 12 * 0) >> 8 = v`.

4. **Ghost table reads from `ca_temp` which is only populated when `glass_intensity > 0`**:
   Both ghost and CA are gated on `glass_intensity > 0`, so `ca_temp` is always populated
   before ghost reads it. **Mitigation**: Already safe by construction. Add a debug_assert.

5. **Division in ghost inner loop**: `/ 100` is a slow operation in the hot loop.
   **Mitigation**: Pre-compute `alpha_shift = (opacity * gi) / 100` per pixel, then use
   `>> 8` for the channel blend. Only one division per active pixel, not three.

6. **`normalize_tuple` called in build loop (init-only)**: Uses `sqrt` but only at build time
   (630K calls). Takes ~50ms. **Mitigation**: Acceptable for one-time init. The helper's
   half-vectors for the 3 lights can be pre-computed outside the pixel loop (they don't
   depend on pixel position) — compute `H = normalize(L + V)` once per light source.

7. **Glare table range increase (50→80) might look too bright on existing intensity settings**:
   **Mitigation**: The denominator change (20000→25000) compensates. At glass_intensity=60,
   brightness=0: old max = `50 * 60 * 200 / 20000 = 30`, new max = `80 * 60 * 200 / 25000
   = 38`. Slightly brighter at peak specular but the peaks are now physically positioned, so
   they look natural rather than overdone.

---

## TESTING STRATEGY

1. **Visual regression**: Screenshot before/after at glass_intensity = 0, 30, 60, 100.
   At intensity 0: output must be pixel-identical to current (all new effects are gated).

2. **LUT identity test**: `build_glass_tint_lut(0)` must return `[0, 1, 2, ..., 255]`.

3. **Ghost boundary test**: Ghost shifts must never produce out-of-bounds source reads.
   Verify with `debug_assert!(src_x < SCREEN_W && src_y < SCREEN_H)`.

4. **Performance test**: Time a full render pass with/without new effects. Acceptable
   overhead: <4ms on target hardware.

5. **Specular highlight position**: The primary specular should appear in the upper-left
   quadrant of the screen (matching the ceiling light direction). Visually verify the
   highlights move with glass curvature, not sit in fixed screen-space positions.

6. **Fresnel edge check**: At glass_intensity=100, screen edges should show noticeably
   stronger reflection than center. The difference should be smooth, not a hard ring.
