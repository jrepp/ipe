#!/bin/bash
# Wrapper script for cargo-geiger supply chain security check
# Handles virtual manifests by checking each workspace member

set -euo pipefail

echo "🔍 Running supply chain security checks..."
echo ""

# Get list of workspace members
echo "📦 Discovering workspace members..."
members=$(cargo metadata --format-version 1 --no-deps | jq -r '.workspace_members[]' | cut -d' ' -f1)

if [ -z "$members" ]; then
    echo "❌ No workspace members found"
    exit 1
fi

echo "Found workspace members:"
echo "$members" | sed 's/^/  - /'
echo ""

# Track if any checks fail
all_passed=true
failed_crates=""

# Run cargo-geiger on each member
for member in $members; do
    # Extract crate name (format is "name version (path)")
    crate_name=$(echo "$member" | cut -d' ' -f1)
    echo "🔎 Checking $crate_name..."
    
    # Find the crate's directory
    crate_dir=$(cargo metadata --format-version 1 --no-deps | \
                jq -r ".packages[] | select(.name == \"$crate_name\") | .manifest_path" | \
                xargs dirname)
    
    if [ -z "$crate_dir" ]; then
        echo "  ⚠️  Could not find directory for $crate_name, skipping"
        continue
    fi
    
    # Run cargo-geiger in the crate's directory
    if (cd "$crate_dir" && cargo geiger --all-features --all-targets 2>&1); then
        echo "  ✅ $crate_name passed"
    else
        echo "  ❌ $crate_name failed"
        all_passed=false
        failed_crates="$failed_crates\n  - $crate_name"
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
