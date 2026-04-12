fn function_block<'a>(source: &'a str, fn_name: &str) -> &'a str {
    let marker = format!("pub fn {fn_name}");
    let start = source
        .find(&marker)
        .unwrap_or_else(|| panic!("missing function marker: {marker}"));
    let rest = &source[start..];
    let next_fn = rest
        .char_indices()
        .skip(1)
        .find_map(|(idx, _)| rest[idx..].starts_with("\npub fn ").then_some(idx))
        .unwrap_or(rest.len());
    &rest[..next_fn]
}

#[test]
fn crt_filters_use_multi_row_processing_chunks() {
    let source = include_str!("../src/rendering.rs");

    assert!(
        source.contains("const PAR_ROWS: usize = 16;"),
        "rendering.rs should define PAR_ROWS for coarser processing chunks"
    );

    for fn_name in [
        "crt_filter_full",
        "crt_filter_masked",
        "crt_filter_blurred",
        "crt_filter_basic",
    ] {
        let block = function_block(source, fn_name);
        assert!(
            block.contains("chunks_mut(SCREEN_W * PAR_ROWS)"),
            "{fn_name} should chunk processing by multiple rows"
        );
        assert!(
            !block.contains("chunks_mut(SCREEN_W).enumerate()"),
            "{fn_name} should not dispatch one chunk per screen row"
        );
    }
}

#[test]
fn glass_inner_loop_avoids_row_chunk_vec_allocation() {
    let source = include_str!("../src/rendering.rs");
    let block = function_block(source, "glass_inner_loop");

    assert!(
        block.contains("buffer[buf_start..buf_end].chunks_mut(window_width * PAR_ROWS)"),
        "glass_inner_loop should chunk directly over the window slice"
    );
    assert!(
        !block.contains("collect();"),
        "glass_inner_loop should avoid collecting row slices into a Vec each frame"
    );
}
