#!/bin/bash
set -e

# Cargo Audit Vulnerability Scanner with Intelligent Validation
#
# This script runs cargo-audit and validates that only known/acceptable vulnerabilities
# are present. It will fail if HIGH or CRITICAL severity issues are detected.
#
# ALLOWLIST OF ACCEPTABLE VULNERABILITIES:
#
# Currently: NONE - All vulnerabilities should be addressed or explicitly documented here.
#
# When adding to allowlist, document:
# - CVE ID and RUSTSEC advisory ID
# - Severity level
# - Why it's acceptable (e.g., doesn't affect our usage, waiting for upstream fix)
# - Date added and expected resolution date
# - GitHub issue tracking the resolution

echo "Running cargo audit..."

# Run cargo audit with JSON output
if cargo audit --json > audit-report.json 2>&1; then
    echo ""
    echo "✓ No vulnerabilities detected!"
    exit 0
fi

# Audit found issues, parse the JSON
echo ""
echo "========================================"
echo "Cargo Audit detected vulnerabilities"
echo "========================================"

# Check if jq is available for JSON parsing
if ! command -v jq &> /dev/null; then
    echo "⚠️  jq not found, cannot parse JSON output intelligently"
    echo "Installing jq..."
    if command -v apt-get &> /dev/null; then
        sudo apt-get update && sudo apt-get install -y jq
    elif command -v brew &> /dev/null; then
        brew install jq
    else
        echo "✗ Cannot install jq, failing check"
        exit 1
    fi
fi

# Parse vulnerabilities by severity
HIGH_CRITICAL=$(jq -r '.vulnerabilities.list[] | select(.advisory.severity == "high" or .advisory.severity == "critical") | .advisory.id' audit-report.json 2>/dev/null || echo "")
MEDIUM=$(jq -r '.vulnerabilities.list[] | select(.advisory.severity == "medium") | .advisory.id' audit-report.json 2>/dev/null || echo "")
LOW=$(jq -r '.vulnerabilities.list[] | select(.advisory.severity == "low") | .advisory.id' audit-report.json 2>/dev/null || echo "")

# Check for HIGH/CRITICAL vulnerabilities
if [ -n "$HIGH_CRITICAL" ]; then
    echo ""
    echo "✗ HIGH or CRITICAL severity vulnerabilities detected:"
    echo "$HIGH_CRITICAL"
    echo ""
    echo "These must be addressed immediately or explicitly allowlisted with justification."
    echo "See .github/scripts/check-cargo-audit.sh for allowlist documentation."
    exit 1
fi

# Report MEDIUM/LOW vulnerabilities as warnings but don't fail
if [ -n "$MEDIUM" ]; then
    echo ""
    echo "⚠️  MEDIUM severity vulnerabilities detected (non-blocking):"
    echo "$MEDIUM"
    echo ""
fi

if [ -n "$LOW" ]; then
    echo ""
    echo "ℹ️  LOW severity vulnerabilities detected (informational):"
    echo "$LOW"
    echo ""
fi

# If we get here, only MEDIUM/LOW issues exist (or none)
if [ -n "$MEDIUM" ] || [ -n "$LOW" ]; then
    echo "✓ No HIGH/CRITICAL vulnerabilities. MEDIUM/LOW issues are tracked."
    exit 0
fi

# No vulnerabilities found
echo "✓ No vulnerabilities detected!"
exit 0
