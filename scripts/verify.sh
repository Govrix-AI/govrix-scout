#!/usr/bin/env bash
# Govrix — Verification Script
# Checks that the repo is correctly configured and services respond.
# Usage: ./scripts/verify.sh
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

export PATH="$HOME/.cargo/bin:/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:$PATH"

# ── Colors ────────────────────────────────────────────────────────────────────
GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'

PASS=0; FAIL=0; SKIP=0

check() {
    local name="$1" cmd="$2"
    if eval "$cmd" &>/dev/null 2>&1; then
        printf "  ${GREEN}PASS${NC}  %s\n" "$name"
        PASS=$((PASS + 1))
    else
        printf "  ${RED}FAIL${NC}  %s\n" "$name"
        FAIL=$((FAIL + 1))
    fi
}

skip() {
    local name="$1" reason="$2"
    printf "  ${YELLOW}SKIP${NC}  %s  (%s)\n" "$name" "$reason"
    SKIP=$((SKIP + 1))
}

# ── Find repo root ────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
cd "$REPO_DIR"

printf "\n${BOLD}Govrix — Verification${NC}\n"
printf "═══════════════════════════════════════════════════════════════\n\n"

# ── 1. Source files ───────────────────────────────────────────────────────────
printf "${BLUE}[1/5]${NC} Source files\n"
check "Cargo.toml workspace"         "test -f Cargo.toml"
check "govrix.default.toml"          "test -f config/govrix.default.toml"
check "docker-compose.yml"           "test -f docker/docker-compose.yml"
check "Dockerfile"                   "test -f docker/Dockerfile"
check "Dockerfile.dashboard"         "test -f docker/Dockerfile.dashboard"
check "install.sh exists"            "test -f install.sh"
check "install.sh executable"        "test -x install.sh"
check "scripts/verify.sh"            "test -f scripts/verify.sh"
echo ""

# ── 2. No legacy agentmesh references ────────────────────────────────────────
printf "${BLUE}[2/5]${NC} Clean naming (no legacy 'agentmesh' references)\n"
check "Dockerfile clean"             "! grep -q agentmesh docker/Dockerfile"
check "docker-compose.yml clean"     "! grep -q agentmesh docker/docker-compose.yml"
check "Makefile clean"               "! grep -q 'AgentMesh' Makefile"
check "install.sh syntax valid"      "bash -n install.sh"
echo ""

# ── 3. Rust unit tests ────────────────────────────────────────────────────────
printf "${BLUE}[3/5]${NC} Rust unit tests\n"
if command -v cargo &>/dev/null; then
    printf "  ${BLUE}...${NC}  Running cargo test --workspace (this takes ~30s)\n"
    TEST_OUT=$(cargo test --workspace --lib --bins 2>&1 || true)
    FAILED_COUNT=$(echo "$TEST_OUT" | grep -c "^FAILED" || true)
    RESULT_LINE=$(echo "$TEST_OUT" | grep "^test result" | tail -1 || true)
    if [ "$FAILED_COUNT" -eq 0 ] && echo "$RESULT_LINE" | grep -q "ok"; then
        printf "  ${GREEN}PASS${NC}  Unit tests: %s\n" "$RESULT_LINE"
        PASS=$((PASS + 1))
    else
        printf "  ${RED}FAIL${NC}  Unit tests: %s\n" "$RESULT_LINE"
        FAIL=$((FAIL + 1))
    fi
else
    skip "Rust unit tests" "cargo not installed"
fi
echo ""

# ── 4. Docker configuration ───────────────────────────────────────────────────
printf "${BLUE}[4/5]${NC} Docker configuration\n"
if command -v docker &>/dev/null && docker info &>/dev/null 2>&1; then
    check "Docker Compose v2"            "docker compose version"
    check "docker-compose.yml valid"     "docker compose -f docker/docker-compose.yml config -q"
    check "Network name is govrix-scout" "grep -q 'govrix-scout' docker/docker-compose.yml"
    check "POSTGRES_DB is govrix"        "grep -q 'POSTGRES_DB: govrix' docker/docker-compose.yml"
else
    skip "Docker checks" "Docker not running"
fi
echo ""

# ── 5. Live services (optional) ──────────────────────────────────────────────
printf "${BLUE}[5/5]${NC} Live services (requires 'make docker-up')\n"
if [ "$(docker ps 2>/dev/null | grep -c 'govrix-scout' || true)" -gt 0 ]; then
    check "API health endpoint"      "curl -sSf http://localhost:4001/health"
    check "Dashboard (port 3000)"    "curl -sSf http://localhost:3000"
    check "Proxy port open (4000)"   "curl -s --max-time 2 http://localhost:4000 >/dev/null"
    check "Metrics endpoint"         "curl -sSf http://localhost:4001/metrics"
else
    skip "API health"            "services not running (run: make docker-up)"
    skip "Dashboard"             "services not running"
    skip "Proxy"                 "services not running"
    skip "Metrics"               "services not running"
fi
echo ""

# ── Summary ───────────────────────────────────────────────────────────────────
printf "═══════════════════════════════════════════════════════════════\n"
if [ "$FAIL" -eq 0 ]; then
    printf "${GREEN}${BOLD}All checks passed${NC}  (%d passed, %d skipped)\n\n" "$PASS" "$SKIP"
else
    printf "${RED}${BOLD}%d checks failed${NC}, %d passed, %d skipped\n\n" "$FAIL" "$PASS" "$SKIP"
    exit 1
fi
