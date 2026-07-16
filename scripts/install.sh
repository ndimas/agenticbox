#!/usr/bin/env bash
#
# AgenticBox — one-liner install script
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/morpheus-sh/agenticbox/main/scripts/install.sh | bash
#
# Or locally:
#   bash scripts/install.sh
#
# This script:
#   1. Checks prerequisites (Rust, git)
#   2. Clones the AgenticBox repo (shallow, latest tag)
#   3. Builds only the CLI binary
#   4. Installs it to ~/.cargo/bin/ (or a custom path)
#   5. Confirms the install and shows next steps
#
# Environment variables:
#   AGENTICBOX_DIR   — where to clone the repo (default: ~/.agenticbox-source)
#   AGENTICBOX_BIN   — where to install the binary (default: ~/.cargo/bin)
#   AGENTICBOX_TAG   — specific tag to install (default: latest)
#   AGENTICBOX_NOBUILD — skip build, use pre-built binary (default: unset)
#

set -euo pipefail

# ── Colors ──────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

info()  { printf "${BLUE} →${NC} %s\n" "$*"; }
ok()    { printf "${GREEN} ✓${NC} %s\n" "$*"; }
warn()  { printf "${YELLOW} ⚠${NC} %s\n" "$*"; }
err()   { printf "${RED} ✗${NC} %s\n" "$*"; }
header(){ printf "\n${BOLD}%s${NC}\n" "$*"; }

# ── Fail fast on common issues ──────────────────────────────────────
trap 'printf "\n${RED}Installation aborted.${NC}\n"' ERR
cd "$(mktemp -d)" 2>/dev/null || true  # silence errors if mktemp fails

# ── Platform detection ──────────────────────────────────────────────
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')

case "$OS" in
  linux)   PLATFORM="linux-$ARCH" ;;
  darwin)  PLATFORM="macos-$ARCH" ;;
  mingw*|msys*|cygwin*) PLATFORM="windows-$ARCH" ;;
  *)       warn "Untested platform: $OS. Attempting install anyway." ;;
esac

info "Detected: $OS ($ARCH)"

# ── Prerequisites ───────────────────────────────────────────────────
header "Prerequisites"

if ! command -v git &>/dev/null; then
  err "git is required but not installed."
  err "Install git: https://git-scm.com/downloads"
  exit 1
fi
ok "git found: $(git --version | head -1)"

if ! command -v cargo &>/dev/null; then
  err "Rust/Cargo is required but not installed."
  err "Install Rust: https://rustup.rs"
  info "After installing, restart your shell and re-run this script."
  exit 1
fi
ok "cargo found: $(cargo --version | head -1)"

# ── Configuration ───────────────────────────────────────────────────
AGENTICBOX_DIR="${AGENTICBOX_DIR:-$HOME/.agenticbox-source}"
AGENTICBOX_BIN="${AGENTICBOX_BIN:-$HOME/.cargo/bin}"
AGENTICBOX_TAG="${AGENTICBOX_TAG:-}"
BUILD_DIR="$AGENTICBOX_DIR"

header "Configuration"
info "Source directory: $AGENTICBOX_DIR"
info "Install target:   $AGENTICBOX_BIN/agenticbox"

# ── Check for existing install ──────────────────────────────────────
if command -v agenticbox &>/dev/null; then
  EXISTING_VERSION=$(agenticbox --version 2>/dev/null || true)
  warn "AgenticBox is already installed: $EXISTING_VERSION"
  printf "  Re-install? [Y/n]: "
  read -r REPLY
  case "$REPLY" in
    n|N|no) info "Skipping install."; exit 0 ;;
    *)      info "Re-installing...";;
  esac
fi

# ── Clone or update repo ────────────────────────────────────────────
header "Downloading AgenticBox"

if [ -d "$AGENTICBOX_DIR/.git" ]; then
  info "Updating existing clone..."
  cd "$AGENTICBOX_DIR"
  git fetch --tags --depth=1 origin 2>/dev/null || git fetch --depth=1 origin
  if [ -n "$AGENTICBOX_TAG" ]; then
    git checkout "$AGENTICBOX_TAG" 2>/dev/null || git checkout "tags/$AGENTICBOX_TAG" 2>/dev/null || {
      warn "Tag '$AGENTICBOX_TAG' not found, using main branch"
      git checkout main
    }
  else
    git checkout main
  fi
  ok "Repository updated"
else
  info "Cloning repository..."
  mkdir -p "$(dirname "$AGENTICBOX_DIR")"
  git clone --depth=1 --branch=main \
    https://github.com/morpheus-sh/agenticbox.git \
    "$AGENTICBOX_DIR" 2>&1 | tail -1
  ok "Repository cloned"
fi

cd "$AGENTICBOX_DIR"

# ── Build the CLI ───────────────────────────────────────────────────
header "Building AgenticBox CLI (this may take a minute)"

if [ -n "${AGENTICBOX_NOBUILD:-}" ]; then
  info "Skipping build (AGENTICBOX_NOBUILD is set)"
  # Look for a pre-built binary
  if [ -f "./target/release/agenticbox.exe" ]; then
    BINARY_PATH="./target/release/agenticbox.exe"
  elif [ -f "./target/release/agenticbox" ]; then
    BINARY_PATH="./target/release/agenticbox"
  else
    err "No pre-built binary found. Unset AGENTICBOX_NOBUILD or build manually."
    exit 1
  fi
  ok "Using pre-built binary"
else
  info "Running: cargo build --release --bin agenticbox -p agenticbox-cli"
  cargo build --release --bin agenticbox -p agenticbox-cli 2>&1

  if [ -f "./target/release/agenticbox.exe" ]; then
    BINARY_PATH="./target/release/agenticbox.exe"
  elif [ -f "./target/release/agenticbox" ]; then
    BINARY_PATH="./target/release/agenticbox"
  else
    err "Build succeeded but binary not found. Check target/release/ for the binary."
    exit 1
  fi
  ok "Build complete"
fi

# ── Install ─────────────────────────────────────────────────────────
header "Installing"

mkdir -p "$AGENTICBOX_BIN"

if [ -f "$AGENTICBOX_BIN/agenticbox" ] || [ -f "$AGENTICBOX_BIN/agenticbox.exe" ]; then
  cp "$BINARY_PATH" "$AGENTICBOX_BIN/agenticbox.bak" 2>/dev/null || true
fi

cp "$BINARY_PATH" "$AGENTICBOX_BIN/agenticbox"
chmod +x "$AGENTICBOX_BIN/agenticbox"
ok "Installed to: $AGENTICBOX_BIN/agenticbox"

# ── Verify ──────────────────────────────────────────────────────────
if "$AGENTICBOX_BIN/agenticbox" --version &>/dev/null; then
  VERSION=$("$AGENTICBOX_BIN/agenticbox" --version 2>&1)
  ok "Installed: agenticbox $VERSION"
else
  err "Installation verification failed. The binary may not be functional."
  exit 1
fi

# ── PATH warning ────────────────────────────────────────────────────
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$AGENTICBOX_BIN"; then
  warn "$AGENTICBOX_BIN is not in your PATH"
  printf "\n  Add it to your shell profile:\n"
  printf "    echo 'export PATH=\"\$PATH:%s\"' >> ~/.bashrc\n" "$AGENTICBOX_BIN"
  printf "    source ~/.bashrc\n"
fi

# ── Success ─────────────────────────────────────────────────────────
header "━ AgenticBox installed ━"
printf ""
printf "  ${CYAN}%s${NC}\n" "Watch the permission guard demo:"
printf "    ${BOLD}agenticbox run demo${NC}\n"
printf ""
printf "  ${CYAN}%s${NC}\n" "List available agents:"
printf "    ${BOLD}agenticbox agents${NC}\n"
printf ""
printf "  ${CYAN}%s${NC}\n" "Run a named agent (requires Docker):"
printf "    ${BOLD}agenticbox run security-analyst${NC}\n"
printf ""
printf "  ${CYAN}%s${NC}\n" "Preview permissions without executing:"
printf "    ${BOLD}agenticbox run security-analyst --dry-run${NC}\n"
printf ""
printf "  ${CYAN}%s${NC}\n" "See the audit trail:"
printf "    ${BOLD}agenticbox audit --summary${NC}\n"
printf "    ${BOLD}agenticbox audit --verify${NC}\n"
printf ""
printf "  ${CYAN}%s${NC}\n" "Source code:"
printf "    ${BOLD}%s${NC}\n" "$AGENTICBOX_DIR"
printf ""
ok "AgenticBox is ready."
