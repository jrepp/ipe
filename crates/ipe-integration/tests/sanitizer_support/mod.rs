//! Minimal process environment for isolated test clients and instrumentation.
pub fn environment() -> impl Iterator<Item = (&'static str, std::ffi::OsString)> {
    [
        "TSAN_OPTIONS",
        "ASAN_OPTIONS",
        "LSAN_OPTIONS",
        "ASAN_SYMBOLIZER_PATH",
        // Windows networking may load system components through this directory.
        // Preserve the OS location, never provider credentials or proxy settings.
        #[cfg(windows)]
        "SystemRoot",
    ]
    .into_iter()
    .filter_map(|key| std::env::var_os(key).map(|value| (key, value)))
}
