//! Runtime instrumentation is an explicit exception to test credential isolation.
pub fn environment() -> impl Iterator<Item = (&'static str, std::ffi::OsString)> {
    ["TSAN_OPTIONS", "ASAN_OPTIONS", "LSAN_OPTIONS", "ASAN_SYMBOLIZER_PATH"]
        .into_iter()
        .filter_map(|key| std::env::var_os(key).map(|value| (key, value)))
}
