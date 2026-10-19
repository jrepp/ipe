#!/bin/bash
# Wrapper script for sanitizer checks
# A sanitizer run passes only when the command and report checks succeed.

set -euo pipefail

SANITIZER_TYPE="${1:-address}"
sanitizer_target="${2:-x86_64-unknown-linux-gnu}"
case "$sanitizer_target" in
    x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu|aarch64-apple-darwin) ;;
    *) echo "Unsupported sanitizer target: $sanitizer_target" >&2; exit 2 ;;
esac
case "$SANITIZER_TYPE" in
    address|leak|thread) ;;
    *) echo "Unsupported sanitizer: $SANITIZER_TYPE" >&2; exit 2 ;;
esac

# Keep each invocation separate: old findings cannot contaminate a new run and
# diagnostics survive both failures and the shell's exit for CI artifact upload.
report_root="${IPE_SANITIZER_REPORT_ROOT:-$PWD/target/sanitizer-reports}"
mkdir -p "$report_root"
report_root=$(cd "$report_root" && pwd)
report_dir=$(mktemp -d "$report_root/$SANITIZER_TYPE.XXXXXX")
echo "Sanitizer evidence: $report_dir"

echo "🧹 Running $SANITIZER_TYPE sanitizer tests..."

# Run sanitizer
export RUSTFLAGS="-Z sanitizer=$SANITIZER_TYPE"
export RUSTDOCFLAGS="-Z sanitizer=$SANITIZER_TYPE"
export ASAN_OPTIONS="detect_leaks=1"
# Absolute path: newer LLVM exits non-zero when the suppressions file
# cannot be read, which masked real results with a false failure.
LSAN_SUPPRESSIONS="suppressions=$PWD/lsan-suppressions.txt:fast_unwind_on_malloc=0"
# Standalone LSan (from `-Zsanitizer=leak`) ignores ASAN_SYMBOLIZER_PATH;
# its options parser only picks up external_symbolizer_path from
# LSAN_OPTIONS itself. Final LSAN_OPTIONS is assembled after the
# symbolizer search below.
# Symbolize sanitizer reports: without a symbolizer, LSan suppression
# rules (which match symbol names like `leak:rocksdb`) cannot be applied
# and stacks print as raw addresses.
symbolizer=$(command -v llvm-symbolizer || true)
if [ -z "$symbolizer" ]; then
    rustup component add llvm-tools --toolchain nightly >/dev/null 2>&1 || true
    symbolizer=$(find "$(rustc +nightly --print sysroot)" -name 'llvm-symbolizer*' -type f 2>/dev/null | head -n1 || true)
fi
if [ -z "$symbolizer" ]; then
    for candidate in /usr/lib/llvm-*/bin/llvm-symbolizer; do
        [ ! -f "$candidate" ] || symbolizer="$candidate"
    done
fi
if [ -n "$symbolizer" ]; then
    export ASAN_SYMBOLIZER_PATH="$symbolizer"
    export LSAN_OPTIONS="$LSAN_SUPPRESSIONS:external_symbolizer_path=$symbolizer"
    echo "🔤 Sanitizer symbolizer: $symbolizer"
else
    export LSAN_OPTIONS="$LSAN_SUPPRESSIONS"
    echo "⚠️  No llvm-symbolizer found; reports will not symbolicate"
fi
# The Rust test harness can swallow sanitizer stderr even when tests pass.
# Read the file reports independently of cargo's captured output.
if [ "$SANITIZER_TYPE" = "thread" ]; then
    export TSAN_OPTIONS="log_path=$report_dir/tsan-report"
    if [ -n "$symbolizer" ]; then
        # MCP fixtures intentionally clear PATH; use a concrete tool path so
        # their inherited sanitizer settings can still produce useful stacks.
        export TSAN_OPTIONS="$TSAN_OPTIONS:external_symbolizer_path=$symbolizer"
    fi
    # RUSTFLAGS cannot instrument native dependencies. RocksDB publishes shared
    # initialization state using C++ atomics; TSan must see those operations too.
    # cc-rs target variables cover the native libraries without changing the
    # ordinary production build. Rust supplies the sanitizer runtime at link time.
    native_target="${sanitizer_target//-/_}"
    cflags_name="CFLAGS_$native_target"
    cxxflags_name="CXXFLAGS_$native_target"
    native_flags="-fsanitize=thread -fno-omit-frame-pointer"
    if [[ "$sanitizer_target" == aarch64-* ]]; then
        # Keep the production CRC workaround even when overriding target flags
        # from .cargo/config.toml to add native sanitizer instrumentation.
        native_flags="$native_flags -U__ARM_FEATURE_CRC32"
    fi
    export "CC_$native_target=clang" "CXX_$native_target=clang++"
    export "$cflags_name=${!cflags_name:-} $native_flags"
    export "$cxxflags_name=${!cxxflags_name:-} $native_flags"
    # Never accidentally link a prebuilt, uninstrumented RocksDB via LIB_DIR.
    export ROCKSDB_COMPILE=true
    export SNAPPY_COMPILE=true
fi
output_file="$report_dir/cargo.log"
run_status=0
if cargo +nightly test -Z build-std --target "$sanitizer_target" --all-features 2>&1 | tee "$output_file"; then
    run_status=0
else
    run_status=$?
fi
printf '%s\n' "$run_status" > "$report_dir/exit-status.txt"

report_found=0
for report in "$report_dir"/tsan-report.*; do
    [ -f "$report" ] || continue
    echo ""
    echo "=== $report ==="
    cat "$report"
done
if grep -qE 'WARNING: ThreadSanitizer:|ERROR: (AddressSanitizer|LeakSanitizer|ThreadSanitizer):|SUMMARY: (AddressSanitizer|LeakSanitizer|ThreadSanitizer):|FATAL: ThreadSanitizer:|ThreadSanitizer: reported' "$output_file" "$report_dir"/tsan-report.* 2>/dev/null; then
    report_found=1
fi

# Functional test counts cannot override an abnormal sanitizer exit or report.
# Compilation errors and missing suppressions are failures too: neither proves
# that the sanitizer actually checked the intended code.
if [ "$run_status" -ne 0 ] || [ "$report_found" -ne 0 ]; then
    echo "FAIL: sanitizer command exited $run_status; findings=$report_found"
    echo "Evidence retained in $report_dir"
    exit 1
fi
echo "PASS: sanitizer command and report checks passed"
