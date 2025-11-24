use std::sync::LazyLock;

pub static SYSTEM_PALLETE: LazyLock<[(u8, u8, u8); 64]> = LazyLock::new(|| {
    let bytes = include_bytes!("../../palettes/Composite Direct (FBX).pal");

    let colors: Vec<(u8, u8, u8)> = bytes
        .chunks(3)
        .take(64)
        .map(|rgb| (rgb[0], rgb[1], rgb[2]))
        .collect();

    colors.try_into().unwrap()
});
