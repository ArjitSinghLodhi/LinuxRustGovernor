use std::{fs, path::Path, thread, time::Duration};

use sysinfo::{CpuRefreshKind, RefreshKind, System};

use crate::backend::{Config, FilePaths, GovernorState, PowerManager};

pub fn monitor_handling(config: Config) {
    let paths = FilePaths::config_file_paths().unwrap();
    let mut state = GovernorState::new();
    let mut sys =
        System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()));
    loop {
        sys.refresh_cpu_usage();
        let cpus = sys.cpus();
        state.add_load(if cpus.is_empty() {
            0.0
        } else {
            cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
        });
        let is_ac = PowerManager::get_ac_status(&paths);
        state.last_ac_status = Some(is_ac);
        let real_gov = fs::read_to_string(&paths.governor[0].join("scaling_governor"))
            .unwrap_or_else(|_| "Unknown".into());

        let real_epp =
            fs::read_to_string(&paths.cpu_paths[0].join("energy_performance_preference"))
                .unwrap_or_else(|_| "Unknown".into());

        let turbo_val = fs::read_to_string(&paths.boost_paths[0].join("no_turbo"))
            .unwrap_or_else(|_| "Unknown".into());
        let turbo_status = turbo_val.trim();
        println!("=== RustGovernor Monitor [v1.1.2] ===");
        println!(
            "Source: [{}] | Avg Load: {:.2}%",
            if is_ac { "AC" } else { "DC" },
            state.avg_load
        );
        println!("Governor:  {}", real_gov.trim());
        println!("EPP:       {}", real_epp.trim());
        println!("Turbo:     {}", turbo_status);
        // Print Custom Slots
        let custom_vec = if is_ac {
            &config.ac_custom
        } else {
            &config.dc_custom
        };
        if !custom_vec.is_empty() {
            println!("\n[ Custom Slots ]");
            for slot in custom_vec {
                if slot.folder_path.is_empty() {
                    continue;
                }

                // Get the target value based on current load
                let active_val_opt = slot
                    .thresholds
                    .iter()
                    .filter(|(t, _)| state.avg_load >= *t)
                    .last()
                    .map(|(_, v)| v);
                let active_val = match active_val_opt {
                    Some(val)  => val,
                    None => {continue;},
                };
                let mut paths_to_check = Vec::new();

                // Collect all applicable paths (handles subfolders or direct)
                if slot.subfolder_check == "1" {
                    if let Ok(entries) = fs::read_dir(&slot.folder_path) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if p.is_dir() {
                                let full = p.join(&slot.file_name);
                                if full.exists() {
                                    paths_to_check.push(full);
                                }
                            }
                        }
                    }
                } else {
                    let full = Path::new(&slot.folder_path).join(&slot.file_name);
                    if full.exists() {
                        paths_to_check.push(full);
                    }
                }

                // Verify every path and collect specific errors
                let mut success_count = 0;
                let mut slot_errors = Vec::new();

                for path in &paths_to_check {
                    match fs::read_to_string(path) {
                        Ok(content) => {
                            if content.trim() == active_val.trim() {
                                success_count += 1;
                            } else {
                                let display_path = format_path_display(&path);
                                slot_errors.push(format!(
                                    "Mismatch in \"{}\": Found '{}' expected '{}'",
                                    display_path,
                                    content.trim(),
                                    active_val.trim()
                                ));
                            }
                        }
                        Err(e) => {
                            let display_path = format_path_display(&path);
                            // Specifically check for permission issues
                            if e.kind() == std::io::ErrorKind::PermissionDenied {
                                slot_errors.push(format!(
                                    "ReadErr in {:?}: Permission Denied. Please run with sudo.",
                                    display_path
                                ));
                            } else {
                                slot_errors.push(format!(
                                    "ReadErr in {:?}: {}",
                                    display_path,
                                    e
                                ));
                            }
                        }
                    }
                }

                // Print Summary Line
                let total = paths_to_check.len();
                let status_text = if total == 0 {
                    "(File Not Found)".to_string()
                } else if success_count == total {
                    format!("(Successful {}/{})", success_count, total)
                } else {
                    format!("(Failed {}/{})", total - success_count, total)
                };

                println!(
                    "Slot {:02}: [{}] -> {} {}",
                    slot.slot_id, slot.file_name, active_val, status_text
                );

                // Print Detailed Errors (ReadErr / Mismatch) only if they exist
                for err in slot_errors {
                    println!("      ┗━> [!] {}", err);
                }
            }
        }

        println!("\n[!] Press Ctrl+C to exit.");
        thread::sleep(Duration::from_secs(1));
    }
}

fn format_path_display(path: &std::path::Path) -> String {
    let file_name = path
        .file_name()
        .map(|os_str| os_str.to_string_lossy())
        .unwrap_or_default();

    let parent_name = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|os_str| os_str.to_string_lossy())
        .unwrap_or_default();

    if parent_name.is_empty() {
        file_name.into_owned()
    } else {
        format!("{}/{}", parent_name, file_name)
    }
}
