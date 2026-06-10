# RustGovernor (Linux Edition)

A lightweight, no bloat hardware orchestrator written in Rust. Designed specifically to keep gaming laptops cool without the bloat of Python or redundant Makefile installation or limiting you to just hard-coded paths.

Unlike traditional governors, this tool doesn't just manage your CPU—it allows you to link **any** system file to your real-time CPU load.

I was first trying to port my Windows 11 version of this, then it turned into custom slot version.

## ✨ Features

- **Blazing Fast**: Written in pure Rust. Minimal CPU/RAM footprint.
- **255 Custom Automation Slots**: Map any hardware control (Fans, GPU, Backlight, TDP) to CPU load via `config.txt`.
- **Monitor Mode**: A live dashboard that reads directly from the hardware to show you the *actual* state including everything written in config.txt and custom slots `rustgovernor --monitor`.
- **EPP, Governor & Turbo Control**: Dynamic switching of Energy Performance Preferences, scaling_governor and Intel Turbo Boost by default.
- **AC/DC Detection**: Automatically flips your entire hardware profile when you plug or unplug your charger.
- **Safety First**: Single instance protection for `--run` and root-user validation.
- **Config** This is the main part, it by default only managed EPP, Governor and turbo boost (Intel by default) but you can also write in the custom slots.

## Installation
Requires root priviliges.

1. First download the rustgovernor binary and install.sh
2. Then run install.sh with root it'll copy the binary to /usr/local/bin and setup the service for you.
3. Then you can reboot and let it run

### Uninstallation Instructions
```
# Stop it and prevent it from starting again
sudo systemctl disable --now rustgovernor

# Physically delete the service file
sudo rm /etc/systemd/system/rustgovernor.service

# Reload systemd so it forgets the service existed
sudo systemctl daemon-reload

# Remove the binary and config
sudo rm /usr/local/bin/rustgovernor
sudo rm -rf /etc/RustGovernor
```

## Config

For any changes or custom configurations you can edit the config.txt located in /etc/RustGovernor/config.txt and edit the configuration or add more parameters like this.

`dc_` or `ac_` <= this is the state in which it'll be applied AC (Wall power) or DC (Battery).

The number between state and type is the cpu usage which it'll be applied around for example `ac_15_epp=80` this will apply epp 80 when cpu usage is below 15 percent and above the value before 15 set in config.txt.

The type is the parameter that it'll be applied to for example `ac_45_governor=1` if cpu usage is around 45 it'll apply value 1 to cooling parameter.

Lastly the value after the '=' sign is the value that's going to be applied for example `ac_70_turbo=1` this will apply when the system is running on AC and  below 70 percent CPU usage above the value of the next parameter lower than it then set turbo to 1 (Enabled).

### Custom Slots

As for the custom slots, thats already explained in config.txt for those who know their hardware and want more control
the path is `/etc/RustGovernor/config.txt` by default generated after you start rustgovernor with --run flag.

Its main purpose is to allow you to connect anything with your average CPU load percentage

it also has sub_check so you can write to all the subfolders in the directory which have the specified file and write the same value to all of them with one slot instead of writing each slot manually for each folder if they're in the same directroy with same files

## ⚖️ License & Credits

This project is licensed under the **MIT License**. 

**Disclaimer:** This is shared "as-is" for personal use. It modifies low-level hardware (TDP, fans, etc.). I am not responsible for any hardware damage or instability. **Use at your own risk.**

### Third-Party Attribution
The pre-compiled binary includes code from the following open-source projects, used under their respective MIT/Apache-2.0 licenses:
- **clap**: (c) Kevin K. et al.
- **sysinfo**: (c) Guillaume Gomez
- **single-instance**: (c) hhow0627

