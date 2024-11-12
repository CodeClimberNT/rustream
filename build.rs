fn main() {
    // Re-run build script if build.rs is changed
    println!("cargo:rerun-if-changed=build.rs");

    // Set FFmpeg configure flags for static linking
    std::env::set_var(
        "FFMPEG_CONFIGURE_FLAGS",
        "--disable-doc --disable-programs --enable-static --disable-shared",
    );

    // Build FFmpeg using ffmpeg-next crate's build helper
    ffmpeg_next::build::build().unwrap();
}
