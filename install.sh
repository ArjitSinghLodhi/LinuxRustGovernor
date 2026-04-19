pkill -9 -f rustgovernor
sudo rm /usr/local/bin/rustgovernor
sudo cp rustgovernor /usr/local/bin/
sudo chmod +x /usr/local/bin/rustgovernor

# 3. Create Systemd Service
cat <<EOF | sudo tee /etc/systemd/system/rustgovernor.service
[Unit]
Description=RustGovernor Thermal Management and Custom slot files management
After=multi-user.target

[Service]
Type=simple
ExecStart=/usr/local/bin/rustgovernor --run
Restart=always
User=root

[Install]
WantedBy=multi-user.target
EOF



# 4. Enable and Start
sudo systemctl daemon-reload
sudo systemctl enable --now rustgovernor
echo "RustGovernor installed and started!"

