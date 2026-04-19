use std::{fs, path::Path, thread, time::Duration};

use sysinfo::{CpuRefreshKind, RefreshKind, System};

use crate::backend::{Config, FilePaths, GovernorState, PowerManager};

pub fn monitor_handling() {
    let config = Config::load().unwrap();
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
            .unwrap_or_else(|_| "1".into());
        let turbo_status = turbo_val.trim();
        println!("=== RustGovernor Monitor [v1.0.0] ===");
        println!(
            "Source: [{}] | Avg Load: {:.2}%",
            if is_ac { "AC" } else { "DC" },
            state.avg_load
        );
        println!("Governor:  {}", real_gov.trim());
        println!("EPP:       {}", real_epp.trim());
        println!("Turbo:     {}", turbo_status);
        // 2. Print Custom Slots
        let custom_vec = if is_ac {
            &config.ac_custom
        } else {
            &config.dc_custom
        };
        if !custom_vec.is_empty() {
            println!("\n[ Custom Slots ]");
            for slot in custom_vec {
                if !slot.folder_path.is_empty() {
                    let path = Path::new(&slot.folder_path);
                    // Find what value is currently "active" for this slot
                    let active_val = slot
                        .thresholds
                        .iter()
                        .filter(|(t, _)| state.avg_load >= *t)
                        .last()
                        .map(|(_, v)| v)
                        .map_or("None", |v| v);
                    let mut full_path= None;
                    if slot.subfolder_check == "1"{
                        let entries = fs::read_dir(slot.folder_path.clone()).unwrap();
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                let fullpath = path.join(slot.file_name.clone());
                                if fullpath.exists() {
                                   full_path = Some(fullpath);
                                }
                            }
                        }
                    } else {
                        full_path = Some(path.join(slot.file_name.clone()));
                    }
                    let full_path = full_path.unwrap();
                    let content = fs::read_to_string(full_path);
                    println!(
                        "Slot {:02}: [{}] -> {} {}",
                        slot.slot_id, slot.file_name, active_val,
                        match content {
                            Ok(_) => if content.unwrap().trim() == active_val.trim() {"(Successful)".to_string()} else {"(Failed (file value and config value are not equal)".to_string()},
                            Err(e) => format!("(Failed {})", e),
                        }
                    );
                }
            }
        }

        println!("\n[!] Press Ctrl+C to exit.");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        thread::sleep(Duration::from_secs(1));
    }
}
