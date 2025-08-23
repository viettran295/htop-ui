use std::{fs, time::Duration};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub tick_rate: Option<Duration>,
    #[serde(default)]
    pub blink_threshold_rate: Option<Duration>,
    #[serde(default)]
    pub cpu_threshold: Option<f32>,
    #[serde(default)]
    pub single_cpu_threshold: Option<f32>,
    #[serde(default)]
    pub mem_threshold: Option<f32>
}

impl AppConfig {
    const TICK_RATE: Duration = Duration::from_millis(100);
    const BLINK_THRESHOLD_RATE: Duration = Duration::from_secs(1);
    const CPU_THRESHOLD: f32 = 10.0;
    const SINGLE_CPU_THRESHOLD: f32 = 50.0;
    const MEM_THRESHOLD: f32 = 20.0;
    
    pub fn new(config_path: &str) -> Self {
        let config_yml = Self::load_config(config_path);
        Self {
            tick_rate: Some(config_yml.tick_rate.unwrap_or(Self::TICK_RATE)),
            blink_threshold_rate: Some(config_yml.blink_threshold_rate.unwrap_or(Self::BLINK_THRESHOLD_RATE)),
            cpu_threshold: Some(config_yml.cpu_threshold.unwrap_or(Self::CPU_THRESHOLD)),
            single_cpu_threshold: Some(config_yml.single_cpu_threshold.unwrap_or(Self::SINGLE_CPU_THRESHOLD)),
            mem_threshold: Some(config_yml.mem_threshold.unwrap_or(Self::MEM_THRESHOLD))
        }
    }
    
    fn load_config(config_path: &str) -> Self {
        let config_str = match fs::read_to_string(config_path){
            Ok(s) => s,
            Err(err) => {
                eprintln!("Error opening config file: {}", err);
                return AppConfig::default();
            } 
        };
        match serde_yml::from_str(&config_str) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Error serializing config file: {}", err);
                return AppConfig::default();
            }
        }
    }
}
