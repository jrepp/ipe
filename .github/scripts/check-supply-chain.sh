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
    # Capture output to show errors if it fails
    output=$(cd "$pkg_dir" && cargo geiger --all-features --all-targets 2>&1) || status=$?

    if [ "${status:-0}" -eq 0 ]; then
        echo "  ✅ $pkg_name passed"
    else
        echo "  ❌ $pkg_name failed"
        echo ""
        echo "Error output:"
        echo "$output" | tail -30
        echo ""
        all_passed=false
        failed_crates="$failed_crates\n  - $pkg_name"
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
