#!/usr/bin/env bash
# Fetch full real-world RELAX NG schemas for stress testing.
# Usage: ./scripts/fetch-schemas.sh
#
# Downloads official schemas that are too large or multi-file to commit to the repo.
# Run this once before running `cargo test --test stress` or the full-schema real_world tests.

set -euo pipefail

BASE="$(cd "$(dirname "$0")/.." && pwd)"
RW="$BASE/testdata/real-world"

echo "==> Fetching schemas into $RW"

# ── XHTML 1.0 Strict (full, single-file) ────────────────────────────────────
# Source: James Clark's modular XHTML RELAX NG schemas
# This is the complete schema, not our hand-crafted subset.
XHTML_FULL="$RW/xhtml1-strict-full.rng"
if [ ! -f "$XHTML_FULL" ]; then
  echo "Downloading XHTML 1.0 Strict (full)..."
  curl -fsSL "https://raw.githubusercontent.com/nicferrier/rnc/master/xhtml-strict.rng" \
    -o "$XHTML_FULL" 2>/dev/null || \
  curl -fsSL "http://www.w3.org/MarkUp/SCHEMA/xhtml-strict.rng" \
    -o "$XHTML_FULL" 2>/dev/null || \
  echo "  WARN: Could not download full XHTML schema. Will generate one."
else
  echo "XHTML 1.0 Strict (full): already present"
fi

# ── DocBook 5.1 (multi-file) ────────────────────────────────────────────────
DB_DIR="$RW/docbook51"
DB_MAIN="$DB_DIR/docbook.rng"
if [ ! -f "$DB_MAIN" ]; then
  echo "Downloading DocBook 5.1 schemas..."
  mkdir -p "$DB_DIR"
  DB_ZIP="/tmp/docbook-5.1-rng.zip"
  # DocBook 5.1 schemas from OASIS
  curl -fsSL "https://docbook.org/xml/5.1/rng/docbook.rng" \
    -o "$DB_MAIN" 2>/dev/null || true
  if [ -f "$DB_MAIN" ]; then
    echo "  Downloaded docbook.rng entry point"
    # Download the included modules
    for mod in dbits.rng assembly.rng; do
      curl -fsSL "https://docbook.org/xml/5.1/rng/$mod" \
        -o "$DB_DIR/$mod" 2>/dev/null || true
    done
  else
    echo "  WARN: Could not download DocBook 5.1. Will try alternate sources."
    # Try the docbook.org archive
    curl -fsSL "https://docbook.org/xml/5.0/rng/docbook.rng" \
      -o "$DB_MAIN" 2>/dev/null || \
    echo "  WARN: Could not download DocBook schemas."
  fi
else
  echo "DocBook 5.1: already present"
fi

# ── SVG 1.1 (single-file) ───────────────────────────────────────────────────
SVG_SCHEMA="$RW/svg11.rng"
if [ ! -f "$SVG_SCHEMA" ]; then
  echo "Downloading SVG 1.1 schema..."
  # The SVG 1.1 RELAX NG schema from the W3C
  curl -fsSL "https://raw.githubusercontent.com/nicferrier/rnc/master/svg11.rng" \
    -o "$SVG_SCHEMA" 2>/dev/null || \
  echo "  WARN: Could not download SVG 1.1 schema. Will generate a subset."
else
  echo "SVG 1.1: already present"
fi

echo "==> Done. Check $RW for downloaded schemas."
echo "    Run 'cargo test --test real_world' to test against them."
