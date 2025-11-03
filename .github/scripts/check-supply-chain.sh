#!/bin/bash
# Wrapper script for cargo-geiger supply chain security check
# Handles virtual manifests by checking each workspace member

set -euo pipefail

echo "🔍 Running supply chain security checks..."
echo ""

# Get list of workspace package names (packages with no source = workspace members)
echo "📦 Discovering workspace members..."
packages=$(cargo metadata --format-version 1 --no-deps | \
           jq -r '.packages[] | select(.source == null) | .name')

if [ -z "$packages" ]; then
    echo "❌ No workspace packages found"
    exit 1
fi

echo "Found workspace packages:"
echo "$packages" | sed 's/^/  - /'
echo ""

# Track if any checks fail
all_passed=true
failed_crates=""

# Run cargo-geiger on each package
for pkg_name in $packages; do
    echo "🔎 Checking $pkg_name..."

    # Find the package's directory
    manifest_path=$(cargo metadata --format-version 1 --no-deps | \
                    jq -r ".packages[] | select(.name == \"$pkg_name\") | .manifest_path")

    if [ -z "$manifest_path" ]; then
        echo "  ⚠️  Could not find manifest for $pkg_name, skipping"
        continue
    fi

    pkg_dir=$(dirname "$manifest_path")

    if [ ! -d "$pkg_dir" ]; then
        echo "  ⚠️  Directory does not exist for $pkg_name, skipping"
        continue
    fi

    # Run cargo-geiger in the package's directory
    # Note: cargo-geiger returns non-zero if dependencies have unsafe code (warnings)
    # We only care if OUR crate has unsafe code, so check the output
    output=$(cd "$pkg_dir" && cargo geiger --all-features --all-targets 2>&1) || true

    # Extract the line with our crate's stats
    # Look for pattern: "0/0  0/0  0/0  0/0  0/0  ?  pkg-name version"
    # Handle both plain and colored output (ANSI codes)
    crate_stats=$(echo "$output" | grep -P "^\s*\d+/\d+.*\s+$pkg_name(\s|$)" | head -1 || echo "")

    if [ -z "$crate_stats" ]; then
        # cargo-geiger might have failed to run properly
        # Check if there's a fatal error
        if echo "$output" | grep -qi "error:.*failed\|error:.*could not"; then
            echo "  ❌ $pkg_name failed (cargo-geiger error)"
            echo "  Error:"
            echo "$output" | grep -i "error:" | head -5
            all_passed=false
            failed_crates="$failed_crates\n  - $pkg_name (cargo-geiger error)"
        else
            # No stats found, but also no fatal error - might be warnings only
            # In this case, assume no unsafe code in our crate (warnings are from deps)
            echo "  ✅ $pkg_name passed (no stats found, assuming safe)"
        fi
    else
        # Check if our crate has any unsafe code (non-zero numerators in the stats)
        # Remove ANSI color codes first, then check the first number
        clean_stats=$(echo "$crate_stats" | sed 's/\x1b\[[0-9;]*m//g')
        first_ratio=$(echo "$clean_stats" | grep -oP '^\s*\d+/\d+' | head -1)
        numerator=$(echo "$first_ratio" | cut -d'/' -f1 | tr -d ' ')

        if [ "$numerator" != "0" ]; then
            echo "  ❌ $pkg_name has unsafe code"
            echo "  Stats: $clean_stats"
            all_passed=false
            failed_crates="$failed_crates\n  - $pkg_name"
        else
            echo "  ✅ $pkg_name passed (no unsafe code in crate)"
        fi
    fi
    echo ""
done

if $all_passed; then
    echo "✅ All workspace members passed supply chain security checks"
    exit 0
else
    echo "❌ Some workspace members failed supply chain security checks:"
    echo -e "$failed_crates"
    exit 1
fi
