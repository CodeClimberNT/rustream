fn main() {
    // Re-run build script if build.rs is changed
    println!("cargo:rerun-if-changed=build.rs");

    // Enable backtrace for debugging
    std::env::set_var("RUST_BACKTRACE", "1");

    // Set FFmpeg configure flags for static linking
    std::env::set_var(
        "FFMPEG_CONFIGURE_FLAGS",
        "--disable-doc --disable-programs --enable-static --disable-shared --disable-pic",
    );

    // Specify the path to the FFmpeg libraries
    println!("cargo:rustc-link-search=native=/path/to/ffmpeg/lib");

    // Link to the FFmpeg libraries
    println!("cargo:rustc-link-lib=avcodec");
    println!("cargo:rustc-link-lib=avformat");
    println!("cargo:rustc-link-lib=avutil");
    println!("cargo:rustc-link-lib=swscale");

    // Add any other build-time configurations or tasks here

    // Build FFmpeg using ffmpeg-next crate's build helper
    // ffmpeg_next::build::build().unwrap();
}
