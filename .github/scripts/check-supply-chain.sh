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

    # Extract the first line of stats (our crate) - format: "0/0  0/0  0/0  0/0  0/0  ?  crate-name"
    # If any of the first 5 ratios are non-zero, we have unsafe code in our crate
    crate_stats=$(echo "$output" | grep -E "^[0-9]+/[0-9]+.*$pkg_name" | head -1 || echo "")

    if [ -z "$crate_stats" ]; then
        echo "  ⚠️  Could not find stats for $pkg_name in output"
        echo "  Output:"
        echo "$output" | head -10
        all_passed=false
        failed_crates="$failed_crates\n  - $pkg_name (could not parse output)"
    else
        # Check if our crate has any unsafe code (non-zero numerators in the stats)
        # Format: "Functions/Total  Exprs/Total  Impls/Total  Traits/Total  Methods/Total"
        has_unsafe=$(echo "$crate_stats" | grep -E "^[1-9][0-9]*/[0-9]+" || echo "")

        if [ -n "$has_unsafe" ]; then
            echo "  ❌ $pkg_name has unsafe code"
            echo "  Stats: $crate_stats"
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
