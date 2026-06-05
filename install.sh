#!/usr/bin/env bash
set -e

GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TARGET_PATH="/usr/local/bin/rustgovernor"
SERVICE_PATH="/etc/systemd/system/rustgovernor.service"

printf "${BLUE}=== Starting RustGovernor Installation ===${NC}\n"

if [ -f "$TARGET_PATH" ]; then
    printf "${YELLOW}[!] Warning: RustGovernor binary already exists at %s${NC}\n" "$TARGET_PATH"
    read -p "Do you want to overwrite it? (y/N): " choice
    case "$choice" in 
        [yY][eE][sS]|[yY]) 
            printf "${BLUE}[*] Proceeding with overwrite...${NC}\n"
            ;;
        *)
            printf "${YELLOW}[*] Installation aborted by user.${NC}\n"
            exit 0
            ;;
    esac
fi

printf "${BLUE}[*] Stopping existing services...${NC}\n"
pkill -9 -f rustgovernor || true
sudo rm -f "$TARGET_PATH"

printf "${BLUE}[*] Copying binary to %s...${NC}\n" "$TARGET_PATH"
sudo cp ./rustgovernor "$TARGET_PATH"
sudo chmod +x "$TARGET_PATH"

printf "${BLUE}[*] Creating systemd service file at %s...${NC}\n" "$SERVICE_PATH"
cat <<EOF | sudo tee "$SERVICE_PATH" > /dev/null
[Unit]
Description=RustGovernor Thermal Management and Custom slot files management
After=multi-user.target

[Service]
Type=simple
ExecStart=$TARGET_PATH --run
Restart=always
User=root

[Install]
WantedBy=multi-user.target
EOF

printf "${BLUE}[*] Reloading and starting systemd daemon...${NC}\n"
sudo systemctl daemon-reload
sudo systemctl enable --now rustgovernor

printf "${GREEN}==> RustGovernor successfully installed and started!${NC}\n"
