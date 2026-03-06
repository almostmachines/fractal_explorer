#!/usr/bin/env bash
set -euo pipefail

# Generate a flamegraph from a criterion benchmark.
#
# Usage:
#   ./scripts/flamegraph.sh                              # full pipeline, 800x600 zoom
#   ./scripts/flamegraph.sh "fractal_generation/parallel_rayon/1920x1080/256iter"
#   ./scripts/flamegraph.sh "colour_mapping/fire_gradient/800x600/1024iter" 10
#
# Arguments:
#   $1  benchmark filter (default: full_pipeline/generate_and_map/800x600/1024iter/zoom)
#   $2  profile time in seconds (default: 5)

BENCH_FILTER="${1:-full_pipeline/generate_and_map/800x600/1024iter/zoom}"
PROFILE_TIME="${2:-5}"

OUTPUT_DIR="target/flamegraphs"
mkdir -p "$OUTPUT_DIR"

# Sanitise the filter into a filename
SAFE_NAME=$(echo "$BENCH_FILTER" | tr '/' '_')
OUTPUT_FILE="$OUTPUT_DIR/${SAFE_NAME}.svg"

echo "Profiling: $BENCH_FILTER (${PROFILE_TIME}s)"
echo "Output:    $OUTPUT_FILE"

cargo flamegraph \
    --bench render_pipeline \
    --output "$OUTPUT_FILE" \
    -- --bench "$BENCH_FILTER" --profile-time "$PROFILE_TIME"

# Clean up the large perf.data left in the project root
rm -f perf.data

echo ""
echo "Flamegraph written to $OUTPUT_FILE"
echo "Open in a browser for interactive exploration."
