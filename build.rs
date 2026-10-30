fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    assert_eq!(target_os, "linux", "OpenUUYC Linux requires a Linux target");
    build_neteq();
    build_fonts();
}

fn build_fonts() {
    use std::io::Write;
    let directory = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    for (name, bytes) in [
        ("Hack", epaint_default_fonts::HACK_REGULAR),
        (
            "NotoEmoji-Regular",
            epaint_default_fonts::NOTO_EMOJI_REGULAR,
        ),
        ("Ubuntu-Light", epaint_default_fonts::UBUNTU_LIGHT),
        ("emoji-icon-font", epaint_default_fonts::EMOJI_ICON),
    ] {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(bytes).expect("compress bundled font");
        std::fs::write(
            directory.join(format!("{name}.zlib")),
            encoder.finish().unwrap(),
        )
        .expect("write bundled font");
    }
}

fn build_neteq() {
    let root = std::path::Path::new("vendor/webrtc_neteq");
    println!("cargo:rerun-if-changed=vendor/webrtc_neteq");
    println!("cargo:rerun-if-changed=src/media/audio/neteq_bridge.cc");
    let sources = std::fs::read_to_string(root.join("sources.txt")).expect("NetEq source manifest");
    let mut cpp = cc::Build::new();
    let mut c = cc::Build::new();
    for build in [&mut cpp, &mut c] {
        build
            .opt_level(3)
            .include(root)
            .define("NDEBUG", None)
            .define("RTC_DISABLE_LOGGING", None)
            .define("RTC_DISABLE_TRACE_EVENTS", None)
            .define("WEBRTC_APM_DEBUG_DUMP", "0")
            .define("WEBRTC_OPUS_SUPPORT_120MS_PTIME", "1")
            .warnings(false);
        build
            .define("WEBRTC_POSIX", None)
            .define("WEBRTC_LINUX", None);
    }
    cpp.cpp(true).std("c++17");
    cpp.file("src/media/audio/neteq_bridge.cc");
    for source in sources
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        if source.ends_with(".cc") {
            cpp.file(root.join(source));
        } else if source.ends_with(".c") {
            c.file(root.join(source));
        }
    }
    cpp.compile("openuuyc_neteq");
    c.compile("openuuyc_neteq_dsp");
    println!("cargo:rustc-link-lib=stdc++");
}
