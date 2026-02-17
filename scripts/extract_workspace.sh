#!/usr/bin/env bash
set -euo pipefail

# One-click extractor for the full teaching workspace bundle.
# Usage:
#   bash scripts/extract_workspace.sh
#   bash scripts/extract_workspace.sh /path/to/output_dir

OSNAME="tg-rcore-tutorial"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCHIVE_PATH="${ROOT_DIR}/bundle/${OSNAME}.tar.gz"
OUTPUT_DIR="${1:-${ROOT_DIR}/workspace-full}"

if [[ ! -f "${ARCHIVE_PATH}" ]]; then
  echo "Bundle archive not found: ${ARCHIVE_PATH}" >&2
  echo "Reinstall a version of tg-rcore-tutorial that includes the bundle." >&2
  exit 1
fi

mkdir -p "${OUTPUT_DIR}"
tar -xzf "${ARCHIVE_PATH}" -C "${OUTPUT_DIR}"

echo
echo "Workspace extracted to:"
echo "  ${OUTPUT_DIR}/${OSNAME}"
echo
echo "Next steps:"
echo "  cd \"${OUTPUT_DIR}/${OSNAME}\""
echo "  cargo build -p tg-ch2"
