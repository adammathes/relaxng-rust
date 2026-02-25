#!/usr/bin/env bash
#
# Download real-world RELAX NG schemas for integration testing.
#
# Schemas are placed in testdata/real-world/ which is .gitignored.
# Run this once before running the real-world tests:
#
#   ./testdata/download.sh
#   cargo test --test real_world
#
set -euo pipefail

cd "$(dirname "$0")"
DIR=real-world
mkdir -p "$DIR"

fail=0

fetch() {
  local url="$1" dest="$2"
  if [ -f "$dest" ]; then
    return 0
  fi
  mkdir -p "$(dirname "$dest")"
  echo "  $url"
  if ! curl -fsSL --retry 3 --max-time 60 -o "$dest" "$url"; then
    echo "  FAILED: $url" >&2
    rm -f "$dest"
    fail=1
    return 1
  fi
}

# ── DocBook 5.0 ──────────────────────────────────────────────────────────────
# Official OASIS single-file RELAX NG schema (no includes).
# Source: https://github.com/docbook/cdn/tree/master/schema/5.0/rng
echo "Downloading DocBook 5.0..."
fetch "https://raw.githubusercontent.com/docbook/cdn/master/schema/5.0/rng/docbook.rng" \
      "$DIR/docbook5/docbook.rng"

# ── Atom 1.0 (RFC 4287) ──────────────────────────────────────────────────────
# Mechanical .rng conversion of the RELAX NG Compact schema from RFC 4287
# Appendix B by Tim Dettrick.  Single file, no includes.
echo "Downloading Atom 1.0..."
fetch "https://gist.githubusercontent.com/tjdett/4617547/raw/" \
      "$DIR/atom/atom.rng"

# ── SVG 1.1 ──────────────────────────────────────────────────────────────────
# Official W3C modular RELAX NG schema.
# Source: https://www.w3.org/Graphics/SVG/1.1/rng/
echo "Downloading SVG 1.1 (modular, ~40 files)..."
SVG_BASE="https://www.w3.org/Graphics/SVG/1.1/rng"
SVG_MODULES=(
  svg11.rng
  svg-animation.rng
  svg-animevents-attrib.rng
  svg-basic-clip.rng
  svg-basic-filter.rng
  svg-basic-font.rng
  svg-basic-graphics-attrib.rng
  svg-basic-structure.rng
  svg-basic-text.rng
  svg-clip.rng
  svg-conditional.rng
  svg-container-attrib.rng
  svg-core-attrib.rng
  svg-cursor.rng
  svg-datatypes.rng
  svg-docevents-attrib.rng
  svg-extensibility.rng
  svg-extresources-attrib.rng
  svg-filter.rng
  svg-font.rng
  svg-gradient.rng
  svg-graphevents-attrib.rng
  svg-graphics-attrib.rng
  svg-hyperlink.rng
  svg-image.rng
  svg-marker.rng
  svg-mask.rng
  svg-opacity-attrib.rng
  svg-paint-attrib.rng
  svg-pattern.rng
  svg-profile.rng
  svg-qname.rng
  svg-script.rng
  svg-shape.rng
  svg-structure.rng
  svg-style.rng
  svg-text.rng
  svg-view.rng
  svg-viewport-attrib.rng
  svg-xlink-attrib.rng
)
for mod in "${SVG_MODULES[@]}"; do
  fetch "$SVG_BASE/$mod" "$DIR/svg11/$mod"
done
# Strip <!DOCTYPE ...> lines — the W3C files reference a local DTD that
# won't exist on disk and confuses the XML parser's span arithmetic.
for f in "$DIR"/svg11/*.rng; do
  if grep -q '<!DOCTYPE' "$f" 2>/dev/null; then
    sed -i '/<!DOCTYPE/d' "$f"
  fi
done

# ── XHTML 1.1 ────────────────────────────────────────────────────────────────
# Official W3C modular RELAX NG schema.
# Source: https://www.w3.org/MarkUp/RELAXNG/
echo "Downloading XHTML 1.1 (modular, ~25 files)..."
XHTML_BASE="https://www.w3.org/MarkUp/RELAXNG"
XHTML_MODULES=(
  xhtml11-1.rng
  xhtml-datatypes-1.rng
  xhtml-attribs-1.rng
  xhtml-struct-1.rng
  xhtml-text-1.rng
  xhtml-hypertext-1.rng
  xhtml-list-1.rng
  xhtml-object-1.rng
  xhtml-param-1.rng
  xhtml-pres-1.rng
  xhtml-edit-1.rng
  xhtml-bdo-1.rng
  xhtml-form-1.rng
  xhtml-table-1.rng
  xhtml-image-1.rng
  xhtml-csismap-1.rng
  xhtml-ssismap-1.rng
  xhtml-events-1.rng
  xhtml-meta-1.rng
  xhtml-script-1.rng
  xhtml-style-1.rng
  xhtml-inlstyle-1.rng
  xhtml-link-1.rng
  xhtml-base-1.rng
  xhtml-ruby-1.rng
  xhtml-nameident-1.rng
  xhtml-legacy-1.rng
  xhtml-inputmode-1.rng
  xhtml-target-1.rng
  xhtml-iframe-1.rng
  xhtml-frames-1.rng
  xhtml-applet-1.rng
  xhtml-metaAttributes-1.rng
  xml-events-1.rng
)
for mod in "${XHTML_MODULES[@]}"; do
  fetch "$XHTML_BASE/$mod" "$DIR/xhtml11/$mod"
done

echo ""
if [ "$fail" -eq 0 ]; then
  echo "All schemas downloaded to testdata/$DIR/"
  echo "Run: cargo test --test real_world"
else
  echo "Some downloads failed.  Re-run this script to retry." >&2
  exit 1
fi
