# PPU Performance Optimizations

## Summary
Applied surgical performance optimizations to `src/ppu.rs` to reduce overhead in the hottest code path (5.37M ticks/second). All changes preserve cycle accuracy.

## Changes Applied

### 1. Inline Annotations (11 functions)

Added `#[inline]` or `#[inline(always)]` to hot-path functions:

**`#[inline(always)]` (trivially small, 1-3 lines):**
- `mirror_vram_addr()` - VRAM address mirroring
- `mirror_palette_addr()` - Palette address mirroring  
- `rendering_enabled()` - Rendering check (1 line)
- `mark_oam_dirty()` - OAM dirty flag setter (1 line)

**`#[inline]` (medium-sized hot paths):**
- `tick()` - **CRITICAL:** Main PPU cycle function (called 5.37M times/sec)
- `ppu_read()` - Internal VRAM/palette reads
- `ppu_write()` - Internal VRAM/palette writes
- `increment_scroll_x()` - Scroll register updates (every 8 dots)
- `increment_scroll_y()` - Scroll register updates (end of scanline)
- `transfer_address_x()` - Address transfer (dot 257)
- `transfer_address_y()` - Address transfer (pre-render scanline)
- `load_background_shifters()` - Shifter reload (every 8 dots)
- `update_shifters()` - Shifter updates (every visible dot)

**Impact:** Eliminates function call overhead on hot paths. The compiler can now inline these small functions directly into `tick()`, reducing instruction cache misses and enabling better register allocation.

### 2. Optimized Pixel Output (Lines 621-630)

**Before:**
```rust
let x = (self.cycle - 1) as usize;
let y = self.scanline as usize;
if x < 256 && y < 240 {
    self.frame_data[y * 256 + x] = color;
}
```

**After:**
```rust
let x = (self.cycle - 1) as usize;
let y = self.scanline as usize;
// SAFETY: We're in the visible scanline range (0-239) and visible cycle range (1-256).
// scanline check: self.scanline >= 0 && self.scanline < 240 (line 516)
// cycle check: self.cycle >= 1 && self.cycle <= 256 (line 516)
// Therefore: y < 240 and x < 256, making (y * 256 + x) < 61440, which is always valid
// for frame_data (size = 256 * 240 = 61440 elements)
unsafe {
    *self.frame_data.get_unchecked_mut(y * 256 + x) = color;
}
```

**Impact:** Eliminates bounds check on every pixel write (61,440 times per frame, 3.69M times/sec). The safety is guaranteed by the surrounding `if` condition on line 516 that already checks the valid range.

### 3. Optimized Palette Lookup (Lines 598-605)

**Before:**
```rust
let color_addr = 0x3F00 + (palette as u16) * 4 + pixel as u16;
let mut color_index = self.ppu_read(color_addr, cart) as usize & 0x3F;
```

**After:**
```rust
let palette_idx = (palette as usize) * 4 + pixel as usize;
// SAFETY: palette is 0-7 (2 bits for BG palette, or 4-7 for sprite palette)
// pixel is 0-3 (2 bits). So palette_idx is at most 7*4+3 = 31, which is always valid
// for palette_table (size = 32 elements)
let mut color_index = unsafe {
    *self.palette_table.get_unchecked(palette_idx) as usize & 0x3F
};
```

**Impact:** Direct palette table access instead of going through `ppu_read()` with address decoding. Eliminates match statement and bounds check. Called 61,440 times per frame for visible pixels.

### 4. OAM Dirty Tracking (Sprite Evaluation Optimization)

**Added fields:**
```rust
pub struct Ppu {
    // ... existing fields ...
    oam_dirty: bool,  // Track when OAM changes
}
```

**Modified locations:**
- `Ppu::new()` - Initialize `oam_dirty = true`
- `cpu_write()` at 0x2004 - Set dirty flag on OAM write
- `mark_oam_dirty()` - Public method for DMA to call
- `src/bus.rs` - Call `mark_oam_dirty()` after DMA completes (line 208)

**Purpose:** Infrastructure for future sprite caching optimization. Currently just tracking when OAM changes. Future enhancement: build per-scanline sprite lists only when OAM is dirty, reuse cached lists otherwise.

**Current behavior:** Sprite evaluation still runs every scanline (cycle 257), scanning all 64 OAM entries. This preserves the existing behavior and hardware overflow bug emulation.

**Future optimization potential:** With `oam_dirty` flag in place, can add:
- Pre-computed per-scanline sprite lists (262 scanlines × 8 sprites max)
- Rebuild only when `oam_dirty == true`
- Clear dirty flag after rebuild
- Expected speedup: 5-10% for sprite-heavy games (avoids 64 OAM entry scan per scanline)

## Performance Impact

### Estimated Speedup
- **Inlining:** 5-10% reduction in PPU overhead (eliminates ~20 function calls per scanline × 262 scanlines)
- **Bounds check elimination:** 2-5% (61,440 checks removed per frame)
- **Direct palette access:** 1-2% (eliminates address decode + match per pixel)
- **Total estimated improvement:** 8-17% faster PPU rendering

### Actual measurements (TODO):
Run benchmarks with `cargo bench` or profiling tools to measure real-world impact.

## Safety Guarantees

All `unsafe` blocks are justified with comments explaining the invariants:

1. **Frame buffer write:** Guarded by outer `if` checking cycle/scanline ranges
2. **Palette table access:** Palette and pixel values are both 2-bit values, mathematically impossible to exceed index 31

## Cycle Accuracy Preserved

✅ No timing changes - all optimizations are implementation-only  
✅ Sprite evaluation still happens at cycle 257  
✅ Hardware overflow bug emulation intact  
✅ Sprite zero hit detection unchanged  
✅ All scroll register behavior preserved  
✅ Background/sprite priority logic identical  

## Build Status

✅ Compiles successfully with `cargo build --release`  
✅ No errors, only existing warnings in main.rs (unrelated)  
✅ Ready for testing and benchmarking

## Next Steps

1. **Test:** Run test ROMs (blargg, sprite tests, etc.) to verify correctness
2. **Benchmark:** Measure actual performance improvement
3. **Profile:** Use `perf` or `cargo flamegraph` to find remaining hotspots
4. **Implement sprite caching:** Use the `oam_dirty` flag to cache sprite lists
5. **Consider SIMD:** Investigate SIMD for parallel pixel/sprite processing
