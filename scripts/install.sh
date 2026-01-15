#!/bin/bash
# gardm installation script
# Installs gardm (gar display manager) system-wide

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

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
    log_error "Please run as root (sudo ./install.sh)"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

log_info "Installing gardm from $PROJECT_DIR"

# Build release binaries
log_info "Building release binaries..."
cd "$PROJECT_DIR"
cargo build --release -p gardmd -p gardm-greeter

# Install binaries
log_info "Installing binaries to /usr/bin..."
install -Dm755 target/release/gardmd /usr/bin/gardmd
install -Dm755 target/release/gardm-greeter /usr/bin/gardm-greeter

# Create config directories
log_info "Creating configuration directories..."
mkdir -p /etc/gardm
mkdir -p /usr/share/gardm/backgrounds

# Install PAM configuration
log_info "Installing PAM configuration..."
install -Dm644 etc/pam.d/gardm /etc/pam.d/gardm

# Install systemd service
log_info "Installing systemd service..."
install -Dm644 etc/gardm.service /usr/lib/systemd/system/gardm.service

# Install default config if not exists
if [ ! -f /etc/gardm/config.toml ]; then
    log_info "Installing default configuration..."
    install -Dm644 etc/config.toml /etc/gardm/config.toml
else
    log_warn "/etc/gardm/config.toml already exists, not overwriting"
fi

# Create sysconfig file (empty, for environment overrides)
if [ ! -f /etc/sysconfig/gardm ]; then
    echo "# Environment variables for gardm" > /etc/sysconfig/gardm
    chmod 644 /etc/sysconfig/gardm
fi

# Reload systemd
log_info "Reloading systemd daemon..."
systemctl daemon-reload

log_info "Installation complete!"
echo ""
echo "To enable gardm as your display manager:"
echo "  1. First, TEST gardm (see scripts/test-gardm.sh)"
echo "  2. Then disable your current display manager:"
echo "     sudo systemctl disable sddm  # or gdm, lightdm, etc."
echo "  3. Enable gardm:"
echo "     sudo systemctl enable gardm"
echo "  4. Reboot"
echo ""
log_warn "ALWAYS keep a terminal/tty login available as fallback!"
