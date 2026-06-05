set -e

GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TARGET_PATH="/usr/local/bin/rustgovernor"
SERVICE_PATH="/etc/systemd/system/rustgovernor.service"

echo -e "${BLUE}=== Starting RustGovernor Installation ===${NC}"

if [ -f "$TARGET_PATH" ]; then
    echo -e "${YELLOW}[!] Warning: RustGovernor binary already exists at $TARGET_PATH${NC}"
    read -p "Do you want to overwrite it? (y/N): " choice
    case "$choice" in 
        [yY][eE][sS]|[yY]) 
            echo -e "${BLUE}[*] Proceeding with overwrite...${NC}"
            ;;
        *)
            echo -e "${YELLOW}[*] Installation aborted by user.${NC}"
            exit 0
            ;;
    esac
fi

echo -e "${BLUE}[*] Stopping existing services...${NC}"
pkill -9 -f rustgovernor || true
sudo rm -f "$TARGET_PATH"

echo -e "${BLUE}[*] Copying binary to $TARGET_PATH...${NC}"
sudo cp ./rustgovernor "$TARGET_PATH"
sudo chmod +x "$TARGET_PATH"

echo -e "${BLUE}[*] Creating systemd service file at $SERVICE_PATH...${NC}"
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

echo -e "${BLUE}[*] Reloading and starting systemd daemon...${NC}"
sudo systemctl daemon-reload
sudo systemctl enable --now rustgovernor

echo -e "${GREEN}==> RustGovernor successfully installed and started!${NC}"
