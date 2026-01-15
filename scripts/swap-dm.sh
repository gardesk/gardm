#!/bin/bash
# Safely swap display managers (e.g., SDDM -> gardm)
# This script includes safety checks and rollback instructions

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

if [ "$EUID" -ne 0 ]; then
    log_error "Please run as root (sudo ./swap-dm.sh)"
    exit 1
fi

echo "============================================"
echo "  gardm Display Manager Swap Utility"
echo "============================================"
echo ""

# Detect current display manager
CURRENT_DM=""
for dm in sddm gdm gdm3 lightdm lxdm; do
    if systemctl is-enabled "$dm" >/dev/null 2>&1; then
        CURRENT_DM="$dm"
        break
    fi
done

if [ -z "$CURRENT_DM" ]; then
    log_warn "Could not detect current display manager"
    echo "Which display manager are you currently using?"
    echo "  1) sddm"
    echo "  2) gdm"
    echo "  3) lightdm"
    echo "  4) other"
    read -p "Selection [1-4]: " choice
    case $choice in
        1) CURRENT_DM="sddm" ;;
        2) CURRENT_DM="gdm" ;;
        3) CURRENT_DM="lightdm" ;;
        *)
            read -p "Enter the service name: " CURRENT_DM
            ;;
    esac
fi

log_info "Detected current display manager: $CURRENT_DM"
echo ""

# Pre-flight checks
log_step "Running pre-flight checks..."

# Check gardm is installed
if [ ! -x /usr/bin/gardmd ]; then
    log_error "gardmd not found at /usr/bin/gardmd"
    log_error "Please run install.sh first"
    exit 1
fi

if [ ! -x /usr/bin/gardm-greeter ]; then
    log_error "gardm-greeter not found at /usr/bin/gardm-greeter"
    log_error "Please run install.sh first"
    exit 1
fi

# Check systemd service exists
if [ ! -f /usr/lib/systemd/system/gardm.service ]; then
    log_error "gardm.service not found"
    log_error "Please run install.sh first"
    exit 1
fi

# Check PAM config
if [ ! -f /etc/pam.d/gardm ]; then
    log_error "PAM configuration not found at /etc/pam.d/gardm"
    log_error "Please run install.sh first"
    exit 1
fi

log_info "All pre-flight checks passed"
echo ""

# Safety warnings
echo "============================================"
echo "  IMPORTANT SAFETY INFORMATION"
echo "============================================"
echo ""
log_warn "Swapping display managers can prevent graphical login!"
echo ""
echo "BEFORE PROCEEDING, ensure you can:"
echo "  1. Switch to a TTY (Ctrl+Alt+F2 through F6)"
echo "  2. Log in via TTY as your user"
echo "  3. Have root/sudo access from TTY"
echo ""
echo "ROLLBACK PROCEDURE (if gardm fails):"
echo "  1. Switch to TTY: Ctrl+Alt+F2"
echo "  2. Log in as your user"
echo "  3. Run: sudo systemctl disable gardm"
echo "  4. Run: sudo systemctl enable $CURRENT_DM"
echo "  5. Reboot: sudo reboot"
echo ""

read -p "Have you tested gardm with test-gardm.sh? [y/N]: " tested
if [ "$tested" != "y" ] && [ "$tested" != "Y" ]; then
    log_warn "It's recommended to test gardm first!"
    log_warn "Run: sudo ./test-gardm.sh"
    echo ""
    read -p "Continue anyway? [y/N]: " continue
    if [ "$continue" != "y" ] && [ "$continue" != "Y" ]; then
        log_info "Aborted. Please test gardm first."
        exit 0
    fi
fi

echo ""
read -p "Ready to swap $CURRENT_DM -> gardm? [y/N]: " confirm
if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    log_info "Aborted by user"
    exit 0
fi

echo ""
log_step "Disabling $CURRENT_DM..."
systemctl disable "$CURRENT_DM"

log_step "Enabling gardm..."
systemctl enable gardm

echo ""
log_info "Display manager swap complete!"
echo ""
echo "============================================"
echo "  NEXT STEPS"
echo "============================================"
echo ""
echo "1. Reboot your system: sudo reboot"
echo "2. gardm should start instead of $CURRENT_DM"
echo ""
echo "If gardm fails to start:"
echo "  - Switch to TTY (Ctrl+Alt+F2)"
echo "  - Run: sudo systemctl disable gardm"
echo "  - Run: sudo systemctl enable $CURRENT_DM"
echo "  - Run: sudo reboot"
echo ""
echo "To check gardm status after reboot:"
echo "  systemctl status gardm"
echo "  journalctl -u gardm -b"
echo ""

read -p "Reboot now? [y/N]: " reboot_now
if [ "$reboot_now" = "y" ] || [ "$reboot_now" = "Y" ]; then
    log_info "Rebooting..."
    reboot
fi
