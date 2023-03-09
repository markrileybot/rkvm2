use std::{fs::File, io::BufReader};
use std::path::Path;
use std::process::exit;

use clap_serde_derive::{
    clap::{self, Parser},
    ClapSerde,
};
use directories::ProjectDirs;
use serde::Serialize;
use rkvm2_proto::Key;

#[derive(Parser)]
#[command(author, version=env!("VERSION_STRING"), about)]
struct Args {
    /// Input files
    input: Vec<std::path::PathBuf>,

    /// Config file.  If config.yml is present, use that.  Otherwise, look in ~/.config/rkvm2/config.yml
    #[arg(short = 'c', long = "config", default_value = "config.yml")]
    config_path: std::path::PathBuf,

    /// Dump the resolved config and exit
    #[arg(short = 'D', long = "dump-config", default_value="false")]
    dump_config: bool,

    /// Rest of arguments
    #[command(flatten)]
    pub config: <Config as ClapSerde>::Opt,
}

#[derive(ClapSerde, Debug, Serialize)]
pub struct Config {
    /// rkvm2 config: The broadcast address to use.  Default 192.168.24.255:45321
    #[arg(short = 'b', long = "broadcast-address")]
    pub broadcast_address: String,

    /// rkvm2 config: The keys to use to switch to the next node.  Default RightCtrl+Tab
    #[arg(short = 's', long = "switch-keys")]
    pub switch_keys: Vec<Key>,

    /// rkvm2 config: The keys to use to switch back to the commander.  Default RightCtrl+RightAlt
    #[arg(short = 'S', long = "switch-commander-keys")]
    pub commander_keys: Vec<Key>,

    /// rkvm2-inputd config: True if this host is the commander.  Default false
    #[arg(short = 'C', long = "commander")]
    pub commander: bool,

    /// rkvm2-inputd config: The GID to use when creating the socket file on linux.  Default 0
    #[arg(short = 'g', long = "socket-gid")]
    pub socket_gid: u32,
}

impl Config {
    pub fn read() -> Config {
        // Parse whole args with clap
        let mut args = Args::parse();

        let config_path = if Path::exists(&args.config_path) {
            args.config_path
        } else {
            match ProjectDirs::from("com", "rkvm2", "rkvm2") {
                None => args.config_path,
                Some(p) => p.config_dir().join("config.yml")
            }
        };

        log::debug!("Attempt to read config from {:?}", config_path);

        // Get config file
        let mut config = if let Ok(f) = File::open(&config_path) {
            // Parse config with serde
            match serde_yaml::from_reader::<_, <Config as ClapSerde>::Opt>(BufReader::new(f)) {
                // merge config already parsed from clap
                Ok(config) => Config::from(config).merge(&mut args.config),
                Err(err) => panic!("Error in configuration file:\n{}", err),
            }
        } else {
            // If there is not config file return only config parsed from clap
            Config::from(&mut args.config)
        };

        // apply defaults
        if config.broadcast_address.is_empty() {
            config.broadcast_address = "192.168.24.255:45321".to_string();
        }
        if config.switch_keys.is_empty() {
            config.switch_keys.push(Key::RightCtrl);
            config.switch_keys.push(Key::Tab);
        }
        if config.commander_keys.is_empty() {
            config.commander_keys.push(Key::RightCtrl);
            config.commander_keys.push(Key::RightAlt);
        }

        if args.dump_config {
            println!("# RKVM2 Config\n\n{}", serde_yaml::to_string(&config).expect("Failed to serialize config"));
            exit(0);
        }

        return config;
    }
}
