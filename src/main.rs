use crate::{
    backend::{
        Config, FilePaths, GovernorState, PowerManager, apply_custom_settings,
        apply_hardware_settings,
    },
    monitor::monitor_handling,
};
use clap::Parser;
use single_instance::SingleInstance;
use std::{process, thread, time::Duration};
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
        eprintln!("Error: This binary is designed for Linux only (to access /sys).");
        process::exit(1);
    }
    let mut sys =
        System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()));
    let config = Config::load().unwrap();
    let paths = FilePaths::config_file_paths().unwrap();
    let mut state = GovernorState::new();
    sys.refresh_cpu_all();
    let flags = [args.run, args.monitor];
    if flags.iter().filter(|&&f| f).count() > 1 {
        eprintln!("Error: Please provide only one flag at a time.");
        std::process::exit(1);
    }
    if args.monitor {
        return monitor_handling();
    }

    if args.run {
        let instance = SingleInstance::new("rustgovernor").unwrap();
        if args.run {
            if !instance.is_single() {
                eprintln!("Error: Another instance of RustGovernor is already running.");
                process::exit(1);
            }
        }
        let uid = std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "1".to_string());
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

            let (mut t_governor, mut t_turbo) = if is_ac {
                ("performance", 0)
            } else {
                ("powersave", 0)
            };
            let mut t_epp = if is_ac {
                "balanced_performance".to_string()
            } else {
                "power".to_string()
            };
            if is_ac {
                for (threshold, val) in &config.ac_governor {
                    if state.avg_load <= *threshold {
                        t_governor = &*val;
                        break;
                    }
                }
                for (threshold, val) in &config.ac_turbo {
                    if state.avg_load <= *threshold {
                        t_turbo = *val;
                        break;
                    }
                }
                for (threshold, val) in &config.ac_epp {
                    if state.avg_load <= *threshold {
                        t_epp = val.clone();
                        break;
                    }
                }
            } else {
                for (threshold, _val) in &config.dc_governor {
                    if state.avg_load <= *threshold {
                        t_governor = &config.dc_cap_governor;
                        break;
                    }
                }
                for (threshold, val) in &config.dc_epp {
                    if state.avg_load <= *threshold {
                        t_epp = val.clone();
                        break;
                    }
                }
            }
            let custom = config.ac_custom.clone();
            //println!("governor: {} epp: {} turbo: {} cooling: {}", t_governor, t_epp, t_turbo, t_cooling);
            let _ = apply_hardware_settings(
                &mut state,
                &paths,
                t_governor.to_string(),
                t_turbo,
                t_epp.clone(),
                is_ac,
                changed,
            );
            apply_custom_settings(&mut state, &custom, changed, is_ac);
            thread::sleep(Duration::from_secs(1));
        }
    }
}
