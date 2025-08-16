pub mod process;

use std::{
    thread, 
    sync::mpsc::Sender,
    time::Duration
};
use sysinfo::{System, Users, MINIMUM_CPU_UPDATE_INTERVAL};

pub fn list_all_processes(tx: Sender<Vec<process::Process>>){
    let mut sys = System::new_all();
    let users = Users::new_with_refreshed_list();
    let total_mem = sys.total_memory();
    let mut mem_usage = 0.0;
    let mut cpu_usage = 0.0;

    thread::spawn(move || {
        sys.refresh_all();
        thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
        loop {
            sys.refresh_all();
            let mut vec_proc: Vec<process::Process> = Vec::new();               
            for (pid, process) in sys.processes() {
                let user_id = process.user_id().unwrap();
                let user = users.get_user_by_id(user_id).unwrap().name();
                mem_usage = (process.memory() as f32 / total_mem as f32) * 100.0;
                cpu_usage = process.cpu_usage() / sys.global_cpu_usage();
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
            tx.send(vec_proc).unwrap();
            
            thread::sleep(Duration::from_secs(1));
        }
    });
}