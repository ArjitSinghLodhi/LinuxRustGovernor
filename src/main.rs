use crate::{
    backend::{
        Config, FilePaths, GovernorState, PowerManager, apply_custom_settings,
        apply_hardware_settings,
    },
    monitor::monitor_handling,
};
use clap::Parser;
use single_instance::SingleInstance;
use std::{
    process::{self, exit},
    thread,
    time::Duration,
};
use sysinfo::{CpuRefreshKind, RefreshKind, System};
mod backend;
mod monitor;
#[derive(Parser, Debug)]
#[command(version, arg_required_else_help = true)]
struct Args {
    #[arg(
        short,
        long,
        group = "mode",
        help = "Runs RustGovernor with no verbose output, used for service function"
    )]
    run: bool,
    #[arg(
        short,
        long,
        group = "mode",
        help = "Enable detailed monitoring and logging by reading data"
    )]
    monitor: bool,
}

fn main() {
    let args = Args::parse();
    if std::env::consts::OS != "linux" {
        eprintln!("Error: This binary is designed for Linux only.");
        process::exit(1);
    }
    let mut sys =
        System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()));
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Config parsing error: {}", e);
            exit(1)
        }
    };
    let paths = FilePaths::config_file_paths().unwrap();
    let mut state = GovernorState::new();
    sys.refresh_cpu_all();
    let flags = [args.run, args.monitor];
    if flags.iter().filter(|&&f| f).count() > 1 {
        eprintln!("Error: Please provide only one flag at a time.");
        std::process::exit(1);
    }
    if args.monitor {
        return monitor_handling(config);
    }

    if args.run {
        let single_instance_result = SingleInstance::new("rustgovernor_lock");
        let _single_instance = match single_instance_result {
            Ok(instance) => match instance.is_single() {
                true => instance,
                false => {
                    eprintln!("[!] RustGovernor is already running!");
                    exit(1)
                }
            },
            Err(e) => {
                println!("Could not get single_instance: {}", e);
                exit(1)
            }
        };
        let uid = std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());
        if uid != "0" {
            eprintln!("Error: RustGovernor must be run with sudo/root permissions.");
            std::process::exit(1);
        }

        loop {
            sys.refresh_cpu_usage();
            let cpus = sys.cpus();
            state.add_load(if cpus.is_empty() {
                0.0
            } else {
                cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
            });
            let is_ac = PowerManager::get_ac_status(&paths);
            let changed = state.last_ac_status != Some(is_ac);
            state.last_ac_status = Some(is_ac);

            let mut t_governor = Option::None;
            let mut t_turbo = Option::None;
            let mut t_epp = Option::None;
            if is_ac {
                for (threshold, val) in &config.ac_governor {
                    if state.avg_load >= *threshold {
                        t_governor = Some(val);
                    }
                }
                for (threshold, val) in &config.ac_turbo {
                    if state.avg_load >= *threshold {
                        t_turbo = Some(val);
                    }
                }
                for (threshold, val) in &config.ac_epp {
                    if state.avg_load >= *threshold {
                        t_epp = Some(val);
                    }
                }
            } else {
                for (threshold, _val) in &config.dc_governor {
                    if state.avg_load >= *threshold {
                        t_governor = Some(&config.dc_cap_governor);
                    }
                }
                for (threshold, val) in &config.dc_turbo {
                    if state.avg_load >= *threshold {
                        t_turbo = Some(val);
                    }
                }
                for (threshold, val) in &config.dc_epp {
                    if state.avg_load >= *threshold {
                        t_epp = Some(val);
                    }
                }
            }
            let custom = if is_ac {
                &config.ac_custom
            } else {
                &config.dc_custom
            };
            //println!("governor: {} epp: {} turbo: {} cooling: {}", t_governor, t_epp, t_turbo, t_cooling);
            apply_hardware_settings(
                &mut state,
                &paths,
                t_governor.cloned(),
                t_turbo.cloned(),
                t_epp.cloned(),
                is_ac,
                changed,
            );
            apply_custom_settings(&mut state, &custom, changed, is_ac);
            thread::sleep(Duration::from_secs(1));
        }
    }
}
