#!/usr/bin/env bash
# ==============================================================================
# Dead By Queue — GitHub Releases Cleanup & Repackaging Utility
# ==============================================================================
# Usage:
#   ./scripts/cleanup-releases.sh [OPTIONS]
#
# Options:
#   --dry-run             Preview actions without deleting or uploading anything.
#   --delete-legacy       Delete legacy pre-v0.6.0 GitHub releases and remote tags.
#   --repackage-v06       Download existing v0.6.x release binaries, create .tar.gz/.zip
#                         archives with README & LICENSE, generate SHA256SUMS.txt,
#                         and upload them.
#   --all                 Run both --delete-legacy and --repackage-v06.
#   --help, -h            Show this help message.
# ==============================================================================

set -euo pipefail

REPO="${GITHUB_REPOSITORY:-trazxdxne/dbdqueue}"
DRY_RUN=false
DO_DELETE=false
DO_REPACKAGE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --delete-legacy)
      DO_DELETE=true
      shift
      ;;
    --repackage-v06)
      DO_REPACKAGE=true
      shift
      ;;
    --all)
      DO_DELETE=true
      DO_REPACKAGE=true
      shift
      ;;
    -h|--help)
      sed -n '2,15p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [ "$DO_DELETE" = false ] && [ "$DO_REPACKAGE" = false ]; then
  echo "No action specified. Use --delete-legacy, --repackage-v06, or --all."
  echo "Use --help for more information."
  exit 1
fi

# Ensure gh CLI is installed and authenticated when not in dry-run mode
if [ "$DRY_RUN" = false ]; then
  if ! command -v gh >/dev/null 2>&1; then
    echo "Error: GitHub CLI (gh) is required but not installed or not in PATH." >&2
    exit 1
  fi

  if ! gh auth status >/dev/null 2>&1; then
    echo "Error: gh CLI is not authenticated. Please run 'gh auth login' first." >&2
    exit 1
  fi
fi

echo "==> Target repository: $REPO"
if [ "$DRY_RUN" = true ]; then
  echo "==> DRY-RUN MODE: No changes will be committed to GitHub."
fi

# ------------------------------------------------------------------------------
# 1. Delete Legacy Pre-v0.6.0 Releases and Tags
# ------------------------------------------------------------------------------
LEGACY_TAGS=(
  "v0.1.0"
  "v0.1.1"
  "v0.1.2"
  "v0.1.3"
  "v0.1.4"
  "v0.1.5"
  "v0.3.0"
  "v0.3.1"
  "v0.5.0"
  "v0.5.1"
  "v0.5.2"
  "v0.5.3"
  "v0.5.4"
)

if [ "$DO_DELETE" = true ]; then
  echo ""
  echo "----------------------------------------------------------------------"
  echo "==> Processing Legacy Pre-v0.6.0 Releases..."
  echo "----------------------------------------------------------------------"

  for tag in "${LEGACY_TAGS[@]}"; do
    if [ "$DRY_RUN" = true ]; then
      echo "[DRY-RUN] Would delete GitHub release: $tag"
      echo "[DRY-RUN] Would delete remote Git tag: refs/tags/$tag"
    else
      echo "Checking release $tag on $REPO..."
      if gh release view "$tag" --repo "$REPO" >/dev/null 2>&1; then
        echo "Deleting release $tag..."
        gh release delete "$tag" --repo "$REPO" --yes || true
      else
        echo "Release $tag does not exist on GitHub."
      fi

      echo "Deleting remote tag $tag..."
      git push --delete origin "$tag" 2>/dev/null || true
    fi
  done
fi

# ------------------------------------------------------------------------------
# 2. Repackage v0.6.x Releases with Standardized Archives and Checksums
# ------------------------------------------------------------------------------
V06_TAGS=("v0.6.0" "v0.6.1" "v0.6.2")

if [ "$DO_REPACKAGE" = true ]; then
  echo ""
  echo "----------------------------------------------------------------------"
  echo "==> Repackaging v0.6.x Releases (.tar.gz, .zip, SHA256SUMS.txt)..."
  echo "----------------------------------------------------------------------"

  TMP_DIR=$(mktemp -d -t dbdq-repackage-XXXXXX)
  trap 'rm -rf "$TMP_DIR"' EXIT

  cp README.md "$TMP_DIR/"
  cp LICENSE "$TMP_DIR/"

  for tag in "${V06_TAGS[@]}"; do
    echo "==> Processing release: $tag"
    TAG_DIR="$TMP_DIR/$tag"
    mkdir -p "$TAG_DIR"

    if [ "$DRY_RUN" = true ]; then
      echo "[DRY-RUN] Would download binaries for $tag, package archives, and upload to $REPO"
      continue
    fi

    # Check if release exists
    if ! gh release view "$tag" --repo "$REPO" >/dev/null 2>&1; then
      echo "Notice: Release $tag not found on GitHub. Skipping."
      continue
    fi

    # Download existing standalone binaries
    echo "Downloading existing assets for $tag..."
    gh release download "$tag" --repo "$REPO" \
      --pattern "dbdq-*" \
      --dir "$TAG_DIR" || true

    cd "$TAG_DIR"

    # Verify and package Linux x86_64
    if [ -f "dbdq-linux-x86_64" ]; then
      cp dbdq-linux-x86_64 dbdq
      chmod +x dbdq
      tar -czvf "dbdq-linux-x86_64.tar.gz" dbdq "$TMP_DIR/README.md" "$TMP_DIR/LICENSE"
      rm -f dbdq
    fi

    # Verify and package Linux aarch64
    if [ -f "dbdq-linux-aarch64" ]; then
      cp dbdq-linux-aarch64 dbdq
      chmod +x dbdq
      tar -czvf "dbdq-linux-aarch64.tar.gz" dbdq "$TMP_DIR/README.md" "$TMP_DIR/LICENSE"
      rm -f dbdq
    fi

    # Verify and package Windows x64
    if [ -f "dbdq-windows-x64.exe" ]; then
      cp dbdq-windows-x64.exe dbdq.exe
      if command -v 7z >/dev/null 2>&1; then
        7z a -tzip "dbdq-windows-x64.zip" dbdq.exe "$TMP_DIR/README.md" "$TMP_DIR/LICENSE"
      elif command -v zip >/dev/null 2>&1; then
        zip -9 "dbdq-windows-x64.zip" dbdq.exe "$TMP_DIR/README.md" "$TMP_DIR/LICENSE"
      fi
      rm -f dbdq.exe
    fi

    # Generate SHA256SUMS.txt
    sha256sum dbdq* > SHA256SUMS.txt
    echo "Generated SHA256SUMS.txt:"
    cat SHA256SUMS.txt

    # Upload new archive assets and checksums
    echo "Uploading archives and checksums to release $tag..."
    UPLOAD_FILES=()
    [ -f "dbdq-linux-x86_64.tar.gz" ] && UPLOAD_FILES+=("dbdq-linux-x86_64.tar.gz")
    [ -f "dbdq-linux-aarch64.tar.gz" ] && UPLOAD_FILES+=("dbdq-linux-aarch64.tar.gz")
    [ -f "dbdq-windows-x64.zip" ] && UPLOAD_FILES+=("dbdq-windows-x64.zip")
    [ -f "SHA256SUMS.txt" ] && UPLOAD_FILES+=("SHA256SUMS.txt")

    if [ ${#UPLOAD_FILES[@]} -gt 0 ]; then
      gh release upload "$tag" "${UPLOAD_FILES[@]}" --repo "$REPO" --clobber
      echo "Successfully uploaded assets for $tag."
    fi

    cd - >/dev/null
  done
fi

echo ""
echo "==> All requested operations completed successfully!"
