#!/bin/bash
# Test gardm without replacing your current display manager
# This script runs gardm on a different VT/display for testing

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    log_error "Please run as root (sudo ./test-gardm.sh)"
    exit 1
fi

# Find an unused display
find_unused_display() {
    for i in {1..9}; do
        if [ ! -e "/tmp/.X$i-lock" ] && [ ! -S "/tmp/.X11-unix/X$i" ]; then
            echo ":$i"
            return
        fi
    done
    echo ":99"
}

# Find an unused VT (above the typical user range)
find_unused_vt() {
    for vt in {7..12}; do
        if ! fuser "/dev/tty$vt" >/dev/null 2>&1; then
            echo "$vt"
            return
        fi
    done
    echo "8"
}

DISPLAY_NUM=$(find_unused_display)
VT_NUM=$(find_unused_vt)

log_info "Testing gardm on display $DISPLAY_NUM, VT $VT_NUM"
log_warn "Press Ctrl+C here (or switch back to your main VT) to stop the test"
echo ""
echo "To switch VTs:"
echo "  - Ctrl+Alt+F1 through F7 (or F2-F8 depending on your setup)"
echo "  - Your current session is likely on VT 1 or 2"
echo ""
echo "Press Enter to start test, or Ctrl+C to cancel..."
read

# Check if binaries exist
if [ ! -x /usr/bin/gardmd ]; then
    log_error "gardmd not installed. Run install.sh first."
    exit 1
fi

if [ ! -x /usr/bin/gardm-greeter ]; then
    log_error "gardm-greeter not installed. Run install.sh first."
    exit 1
fi

# Check PAM config
if [ ! -f /etc/pam.d/gardm ]; then
    log_error "PAM config not installed. Run install.sh first."
    exit 1
fi

log_info "Starting gardmd on display $DISPLAY_NUM, VT $VT_NUM..."
log_info "You will be switched to VT $VT_NUM"

# Run gardmd directly (not via systemd, for testing)
# The daemon will start X on the specified display/VT
exec /usr/bin/gardmd --display "$DISPLAY_NUM" --vt "$VT_NUM"
