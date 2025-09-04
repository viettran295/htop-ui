pub mod process;
pub mod network;
pub mod disk;
pub mod temperature;
mod utils;

use tokio;
use std::{
    collections::HashMap, sync::mpsc::Sender, time::Duration
};
use sysinfo::{Components, Disks, ProcessStatus, System, Users, MINIMUM_CPU_UPDATE_INTERVAL};

use crate::cmd::{disk::Disk, network::Network, temperature::Temperature, utils::seconds_to_timestamp};

pub enum Message {
    Processes(Vec<process::Process>),
    Network(network::Network),
    CPUUsage(Vec<f32>),
    MEMUsage(f32),
    DiskUsage(Vec<Disk>),
    Temperature(Vec<Temperature>),
    GeneralInfo(Vec<String>),
}

pub fn list_all_processes(tx: Sender<Message>){
    let mut sys = System::new_all();
    let users = Users::new_with_refreshed_list();
    let total_mem = sys.total_memory();

    tokio::spawn(async move {
        sys.refresh_all();
        tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL).await;
        loop {
            sys.refresh_all();
            let mut vec_proc: Vec<process::Process> = Vec::new();
            let total_mem_usage = (sys.used_memory() as f32 / total_mem as f32) * 100.0;
            for (pid, process) in sys.processes() {
                let user_id = process.user_id().unwrap();
                let user = users.get_user_by_id(user_id).unwrap().name();
                let mem_usage = (process.memory() as f32 / total_mem as f32) * 100.0;
                let cpu_usage = process.cpu_usage() / sys.global_cpu_usage();
                if cpu_usage <= 0.0 || mem_usage <= 0.0 {
                    continue;
                }
                let proc = process::Process::default()
                    .set_pid(pid.as_u32())
                    .set_process_name(process.name().to_string_lossy().into_owned())
                    .set_cpu_usage(cpu_usage)
                    .set_mem_usage(mem_usage)
                    .set_user(user.to_string())
                    .build().unwrap();
                vec_proc.push(proc);
            }
            tx.send(Message::Processes(vec_proc)).unwrap();
            tx.send(Message::MEMUsage(total_mem_usage)).unwrap();
            utils::send_cores_usage(&tx, &sys);
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

pub fn get_network_info(tx: Sender<Message>) {
    let mut networks = sysinfo::Networks::new_with_refreshed_list();
    let mut net_data = Network::new();
    
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        loop {
            networks.refresh(true);
            let mut upload_gb = 0.0;
            let mut download_gb = 0.0;
            for (interface, network) in &networks {
                if interface.contains("wlp") || interface.contains("enp") {
                    // To Kilo bits per second
                    upload_gb += network.transmitted() as f64 * 8.0 / 1_000.0;
                    download_gb += network.received() as f64 * 8.0 / 1_000.0;
                    net_data.update(upload_gb, download_gb);
                    tx.send(Message::Network(net_data)).unwrap();
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

pub fn get_disk_usage(tx: Sender<Message>) {
    let sys_disks = Disks::new_with_refreshed_list();
    let mut disks: Vec<Disk> = Vec::new(); 
 
     for disk in sys_disks.list() {
         let disk = Disk::new(
             disk.name().to_string_lossy().into_owned(), 
             disk.total_space(), 
             disk.available_space()
         );
         disks.push(disk);
     }
     tx.send(Message::DiskUsage(disks)).unwrap();
}

pub fn get_temperature(tx: Sender<Message>) {
    let mut temperatures: Vec<Temperature> = Vec::new();
    tokio::spawn(async  move {
        let mut sys_components = Components::new_with_refreshed_list();
        loop {
            temperatures.clear();
            sys_components.refresh(true);
            for comp in sys_components.iter() {
                let temp = Temperature::new(
                    comp.label().to_string(), 
                    comp.temperature().unwrap_or(0.0), 
                    comp.max().unwrap_or(0.0),
                    comp.critical().unwrap_or(0.0),
                );
                if temp.value == 0.0 {
                    continue;
                }
                temperatures.push(temp);
            }
            tx.send(Message::Temperature(temperatures.clone())).unwrap();
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

pub fn get_general_info(tx: Sender<Message>) {
    let mut sys = System::new_all();

    tokio::spawn(async move {
        loop {
            sys.refresh_all();
            let mut message: Vec<String> = Vec::new();
            let mut status_counts: HashMap<ProcessStatus, u32> = HashMap::new();
            let load_avg = System::load_average();
            
            for (_, proc) in sys.processes() {
                *status_counts.entry(proc.status()).or_insert(0) += 1;
            }
            message.push(
                format!("Uptime: {} \n", seconds_to_timestamp(System::uptime()))
            );
            message.push(
                format!("Load averag: 1-minute: {}, 5-minute: {}, 15-minute: {}", load_avg.one, load_avg.five, load_avg.fifteen)
            );
            message.push(
                format!("Tasks: {} total, {} running, {} sleep, {} stopped, {} zombie \n",
                    status_counts.values().sum::<u32>(), 
                    status_counts.get(&ProcessStatus::Run).unwrap_or(&0),
                    status_counts.get(&ProcessStatus::Sleep).unwrap_or(&0),
                    status_counts.get(&ProcessStatus::Stop).unwrap_or(&0),
                    status_counts.get(&ProcessStatus::Zombie).unwrap_or(&0),
                )
            );
            tx.send(Message::GeneralInfo(message)).unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}