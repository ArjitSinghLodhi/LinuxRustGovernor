use anyhow::{Ok, Result, anyhow};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::vec;

const FILE_EPP_NAME: &str = "energy_performance_preference";
#[derive(Clone)]
pub struct CustomSlots {
    pub folder_path: String,
    subfolder_check: String,
    pub file_name: String,
    pub thresholds: Vec<(f32, String)>,
    pub slot_id: u8,
}

pub struct Config {
    pub ac_governor: Vec<(f32, String)>,
    pub ac_turbo: Vec<(f32, u32)>,
    pub ac_epp: Vec<(f32, String)>,
    pub dc_cap_governor: String,
    pub dc_governor: Vec<(f32, String)>,
    pub dc_epp: Vec<(f32, String)>,
    // custom logics.. first path.. then sub folder check or not (1 or 0), then the file to check its name exact.. then value to apply
    pub ac_custom: Vec<CustomSlots>,
    pub dc_custom: Vec<CustomSlots>,
}
impl Config {
    fn default_content() -> &'static str {
        "# --- TEST SLOT: Writing to /tmp/test.txt ---
# Be sure to create the test.txt file in your user folder in Documents or whatever you chose.
#ac_custom1path=/home/user/Documents
#ac_custom1sub_check=0
#ac_custom1file=test.txt
        
# Staircase of values based on load
#ac_0_custom1val=System_is_Idle
#ac_20_custom1val=Light_Work
#ac_50_custom1val=Getting_Hot
#ac_80_custom1val=RUST_GOVERNOR_MAX_LOAD

#Upto 255 number of custom slots, more than enough for anyone

# Slot Format:
# ac_customXpath: The directory containing the file.
# ac_customXfile: The filename to write to.
# ac_customXsub_check: 1 to search subfolders (like policy*), 0 for direct file.
# ac_[load]_customXval: The string to write at that load threshold.

# --- The following are the minimum settings pre-configured ---
# You could delete these if you don't want them because it isn't supported for you or you want everything custom.
ac_15_max=80
ac_20_governor=powersave
ac_30_governor=powersave
ac_40_governor=powersave
ac_50_governor=powersave
ac_60_governor=performance
ac_70_governor=performance
ac_80_governor=performance
ac_90_governor=performance
ac_100_governor=performance
ac_15_turbo=1
ac_50_turbo=1
ac_65_turbo=0
ac_1_epp=power
ac_5_epp=power
ac_10_epp=power
ac_15_epp=power
ac_20_epp=balance_power
ac_40_epp=balance_power
ac_60_epp=balance_performance
ac_80_epp=performance
ac_100_epp=performance
ac_cooling_threshold=45
dc_cap_governor=powersave
dc_15_governor=powersave
dc_40_governor=powersave
dc_60_governor=powersave
dc_100_governor=performance
dc_1_epp=power
dc_15_epp=power
dc_40_epp=power
dc_60_epp=balanced_power"
    }

    pub fn load() -> Result<Self> {
        let config_name = "config.txt";
        let config_dir = Path::new("/etc/RustGovernor");
        let config_path = config_dir.join(config_name);
        if !Path::new(&config_dir).exists() {
            fs::create_dir(config_dir)?;
        }
        if !Path::new(&config_path).exists() {
            let mut f = File::create(&config_path)?;
            f.write_all(Self::default_content().as_bytes())?;
        }

        let mut config = Self {
            ac_governor: vec![],
            ac_turbo: vec![],
            ac_epp: vec![],
            dc_cap_governor: "powersave".to_string(),
            dc_governor: vec![],
            dc_epp: vec![],
            ac_custom: vec![],
            dc_custom: vec![],
        };
        let reader = BufReader::new(File::open(config_path)?);
        for (idx, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() != 2 {
                return Err(anyhow!("Crash: Invalid format on line {}", idx + 1));
            }
            let key = parts[0].trim();
            let val = parts[1].trim();

            if key.starts_with("ac_") && key.ends_with("_governor") {
                let load = key[3..key.len() - 9].parse().unwrap_or(0.0);
                config.ac_governor.push((load, val.parse()?));
            } else if key.starts_with("ac_") && key.ends_with("_turbo") {
                let load: f32 = key[3..key.len() - 6].parse().unwrap_or(0.0);
                config.ac_turbo.push((load, val.parse()?));
            } else if key.starts_with("ac_") && key.ends_with("_epp") {
                let load = key[3..key.len() - 4].parse().unwrap_or(0.0);
                config.ac_epp.push((load, val.to_string()));
            } else if key == "dc_max_cap" {
                config.dc_cap_governor = val.to_string();
            } else if key.starts_with("dc_") && key.ends_with("_governor") {
                let load = key[3..key.len() - 9].parse().unwrap_or(0.0);
                config.dc_governor.push((load, val.to_string()));
            } else if key.starts_with("dc_") && key.ends_with("_epp") {
                let load: f32 = key[3..key.len() - 4].parse().unwrap_or(0.0);
                config.dc_epp.push((load, val.to_string()));
            }
            config
                .ac_governor
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            config
                .ac_turbo
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            config.ac_epp.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            config
                .dc_governor
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            config.dc_epp.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            if key.contains("_custom") {
                if let Some(idx) = key.find("custom") {
                    let after_custom = &key[idx + 6..]; // Skip "custom"

                    // 2. Grab only the numbers immediately following "custom"
                    let id_str: String = after_custom
                        .chars()
                        .take_while(|c| c.is_numeric())
                        .collect();

                    let id: u32 = id_str.parse().unwrap_or(1);
                    let idx_usize = id as usize;
                    let id = idx_usize;

                    let target_vec = if key.starts_with("ac_") {
                        &mut config.ac_custom
                    } else {
                        &mut config.dc_custom
                    };
                    // IMPORTANT: Ensure the vector is big enough
                    while target_vec.len() <= id as usize {
                        target_vec.push(CustomSlots {
                            slot_id: (target_vec.len() as u8), // Store 1-based ID for logs
                            folder_path: String::new(),
                            subfolder_check: "0".to_string(),
                            file_name: String::new(),
                            thresholds: Vec::new(),
                        });
                    }
                    if key.contains("path") {
                        target_vec[id as usize].folder_path = val.to_string();
                    }
                    if key.contains("file") {
                        target_vec[id as usize].file_name = val.to_string();
                    }
                    if key.contains("sub_check") {
                        target_vec[id as usize].subfolder_check = val.to_string();
                    }
                    if key.contains("val") {
                        let parts: Vec<&str> = key.split('_').collect();
                        if parts.len() >= 2 {
                            let load: f32 = parts[1].parse().unwrap_or(0.0);
                            target_vec[id as usize]
                                .thresholds
                                .push((load, val.to_string()));
                        }
                    }
                    if key.starts_with("ac_") {
                        config.ac_custom = target_vec.to_vec()
                    } else {
                        config.dc_custom = target_vec.to_vec()
                    };
                }
            }
            for slot in &mut config.ac_custom {
                slot.thresholds
                    .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            }
            for slot in &mut config.dc_custom {
                slot.thresholds
                    .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            }
        }
        Ok(config)
    }
}
/*
    else if key.starts_with("dc_") && key.ends_with("_custom1path") {
               config.dc_custom1.folder_path = Some(val.to_string());
           } else if key.starts_with("dc_") && key.ends_with("_custom1") {
               let load: f32 = key[3..key.len() - 7].parse().unwrap_or(0.0);
               config.dc_custom1.load = Some(load); config.dc_custom1.val = Some(val.to_string());
           } else if key.starts_with("dc_") && key.ends_with("_custom1sub_check") {
               config.dc_custom1.subfolder_check = Some(val.to_string());
           }
*/
pub struct FilePaths {
    pub cpu_paths: Vec<PathBuf>,
    pub battery_paths: Vec<PathBuf>,
    pub ac: Vec<PathBuf>,
    pub boost_paths: Vec<PathBuf>,
    pub governor: Vec<PathBuf>,
}
impl FilePaths {
    pub fn config_file_paths() -> Result<Self> {
        let battery_path = "/sys/class/power_supply/";
        let battery_file_name = "energy_full";
        let ac_file_name = "online";
        let cpu_path = "/sys/devices/system/cpu/cpufreq/";
        let cpu_file_name = "scaling_governor";
        let intel_boost_path = "/sys/devices/system/cpu/";
        let intel_boost_file = "no_turbo";
        let governor_file_name = "scaling_governor";
        let mut file_paths = Self {
            cpu_paths: vec![],
            battery_paths: vec![],
            ac: vec![],
            boost_paths: vec![],
            governor: vec![],
        };
        file_paths.governor = PowerManager::get_file_path(cpu_path, governor_file_name);
        file_paths.boost_paths = PowerManager::get_file_path(intel_boost_path, intel_boost_file);
        file_paths.ac = FilePaths::get_file_ac(battery_path, ac_file_name);
        file_paths.battery_paths = PowerManager::get_file_path(battery_path, battery_file_name);
        file_paths.cpu_paths = PowerManager::get_file_path(cpu_path, cpu_file_name);
        Ok(file_paths)
    }
    fn get_file_ac(base_path: &str, target_file: &str) -> Vec<PathBuf> {
        let mut results = Vec::new();
        let base = Path::new(base_path);
        // Read only the top-level folders inside the base_path
        if let std::result::Result::Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Look exactly one level down for the file
                    let file_check = path.join(target_file);
                    if file_check.exists() {
                        if let std::result::Result::Ok(content) = fs::read_to_string(&file_check) {
                            if content != "Battery" {
                                results.push(path);
                            }
                        }
                    }
                }
            }
        }
        results
    }
}
pub struct PowerManager;

impl PowerManager {
    pub fn update_setting(paths: &[PathBuf], file_name: &str, value: &str) -> std::io::Result<()> {
        for path in paths {
            let full_path = path.join(file_name);
            //println!("path {:?} value {}", full_path, value.trim());
            fs::write(&full_path, value.trim())?;
        }
        anyhow::Result::Ok(())
    }

    pub fn get_file_path(base_path: &str, target_file: &str) -> Vec<PathBuf> {
        let mut results = Vec::new();
        let base = Path::new(base_path);
        let target_file = target_file.trim();
        // Read only the top-level folders inside the base_path
        if let std::result::Result::Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Look exactly one level down for the file
                    let file_check = path.join(&target_file.trim());
                    if file_check.exists() {
                        results.push(path.clone());
                        //println!("path is {:?} for {}", path, target_file);
                    }
                }
            }
        }
        results
    }
    pub fn get_ac_status(file_config: &FilePaths) -> bool {
        for power_supply in &file_config.ac {
            let power_supply = power_supply.join("online");
            if let std::result::Result::Ok(is_ac) = fs::read_to_string(power_supply) {
                if is_ac.trim() == "1" {
                    return true;
                }
            }
        }
        for power_supply in &file_config.battery_paths {
            let power_supply = power_supply.join("status");
            if let std::result::Result::Ok(is_ac) = fs::read_to_string(power_supply) {
                if is_ac.trim() == "Discharging" {
                    return false;
                }
            }
        }
        true
    }
    pub fn update_custom_setting(
        base_path: &str,
        target_file: &str,
        value: &str,
        sub_check: String,
    ) -> std::io::Result<()> {
        let base = Path::new(base_path);

        if !base.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Base path not found: {}", base_path),
            ));
        }
        if sub_check.trim() == "1" {
            // Case 1: Look inside each subfolder (e.g., policy0, policy1)
            let entries = fs::read_dir(base)?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let full_path = path.join(target_file);
                    if full_path.exists() {
                        fs::write(&full_path, value)?;
                    }
                }
            }
        } else {
            // Case 2: Direct write to the file in the base folder
            let full_path = base.join(target_file);
            if full_path.exists() {
                fs::write(&full_path, value)?;
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Target file not found: {:?}", full_path),
                ));
            }
        }
        std::result::Result::Ok(())
    }
}

pub struct GovernorState {
    pub avg_load: f32,
    pub history: Vec<f32>,
    pub max_history: usize,
    pub last_ac_status: Option<bool>,
    pub last_ac_boost: Option<u32>,
    pub last_ac_governor: Option<String>,
    pub last_ac_epp: Option<String>,
    pub last_dc_boost: Option<u32>,
    pub last_dc_governor: Option<String>,
    pub last_dc_epp: Option<String>,
    pub last_ac_custom: Vec<Option<String>>,
    pub last_dc_custom: Vec<Option<String>>,
}

impl GovernorState {
    pub fn new() -> Self {
        Self {
            avg_load: 0.0,
            history: Vec::new(),
            max_history: 10,
            last_ac_status: None,
            last_ac_boost: None,
            last_ac_governor: None,
            last_ac_epp: None,
            last_dc_boost: None,
            last_dc_governor: None,
            last_dc_epp: None,
            last_ac_custom: vec![None; 256],
            last_dc_custom: vec![None; 256],
        }
    }
    pub fn add_load(&mut self, load: f32) {
        self.history.push(load);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
        self.avg_load = self.history.iter().sum::<f32>() / self.history.len() as f32;
    }
}

pub fn apply_hardware_settings(
    state: &mut GovernorState,
    config: &FilePaths,
    t_governor: String,
    t_turbo: u32,
    t_epp: String,
    is_ac: bool,
    changed: bool,
) {
    if is_ac {
        if state.last_ac_boost != Some(t_turbo) || changed {
            match PowerManager::update_setting(
                &config.boost_paths,
                "no_turbo",
                &t_turbo.to_string(),
            ) {
                ::std::result::Result::Ok(_) => {
                    state.last_ac_boost = Some(t_turbo);
                }
                ::std::result::Result::Err(e) => {
                    eprintln!("[ERROR] Failed to set TURBO: {}", e);
                }
            }
        }
        if state.last_ac_epp != Some(t_epp.clone()) || changed {
            match PowerManager::update_setting(&config.cpu_paths, FILE_EPP_NAME, &t_epp.to_string())
            {
                ::std::result::Result::Ok(_) => {
                    state.last_ac_epp = Some(t_epp.clone());
                }
                ::std::result::Result::Err(e) => {
                    eprintln!("[ERROR] Failed to set EPP: {}", e);
                }
            }
        }
        if state.last_ac_governor != Some(t_governor.clone()) || changed {
            match PowerManager::update_setting(
                &config.governor,
                "scaling_governor",
                &t_governor.to_string(),
            ) {
                ::std::result::Result::Ok(_) => {
                    state.last_ac_governor = Some(t_governor);
                }
                ::std::result::Result::Err(e) => {
                    eprintln!("[ERROR] Failed to set GOVERNOR: {}", e);
                }
            }
        }
    } else {
        if state.last_dc_boost != Some(t_turbo) || changed {
            match PowerManager::update_setting(
                &config.boost_paths,
                "no_turbo",
                &t_turbo.to_string(),
            ) {
                ::std::result::Result::Ok(_) => {
                    state.last_dc_boost = Some(t_turbo);
                }
                ::std::result::Result::Err(e) => {
                    eprintln!("[ERROR] Failed to set TURBO: {}", e);
                }
            }
        }
        if state.last_dc_epp != Some(t_epp.clone()) || changed {
            match PowerManager::update_setting(&config.cpu_paths, FILE_EPP_NAME, &t_epp.to_string())
            {
                ::std::result::Result::Ok(_) => {
                    state.last_dc_epp = Some(t_epp.clone());
                }
                ::std::result::Result::Err(e) => {
                    eprintln!("[ERROR] Failed to set EPP: {}", e);
                }
            }
        }
        if state.last_dc_governor != Some(t_governor.clone()) || changed {
            match PowerManager::update_setting(
                &config.governor,
                "scaling_governor",
                &t_governor.to_string(),
            ) {
                ::std::result::Result::Ok(_) => {
                    state.last_dc_governor = Some(t_governor);
                }
                ::std::result::Result::Err(e) => {
                    eprintln!("[ERROR] Failed to set GOVERNOR: {}", e);
                }
            }
        }
    }
}

pub fn apply_custom_settings(
    state: &mut GovernorState,
    custom_slots: &Vec<CustomSlots>,
    changed: bool,
    is_ac: bool,
) {
    for slot in custom_slots {
        if slot.folder_path.is_empty() || slot.thresholds.is_empty() {
            continue;
        }
        let active_val = slot
            .thresholds
            .iter()
            .filter(|(threshold, _)| state.avg_load >= *threshold)
            .last()
            .map(|(_, val)| val);
        if let Some(target_val) = active_val {
            let id = slot.slot_id as usize;

            // Get the correct state vector based on current power source
            let last_val_vec = if is_ac {
                &mut state.last_ac_custom
            } else {
                &mut state.last_dc_custom
            };

            // Only write if value changed OR power source changed
            if last_val_vec[id] != Some(target_val.clone()) || changed {
                match PowerManager::update_custom_setting(
                    &slot.folder_path,
                    &slot.file_name,
                    target_val,
                    slot.subfolder_check.clone(),
                ) {
                    std::result::Result::Ok(_) => last_val_vec[id] = Some(target_val.clone()),
                    std::result::Result::Err(e) => {
                        eprintln!("[ERROR] Slot {} failed to write: {}", slot.slot_id, e)
                    }
                }
            }
        }
    }
}
