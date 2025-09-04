use std::sync::mpsc::Sender;
use sysinfo::System;

use crate::cmd::Message;

pub fn send_cores_usage(tx: &Sender<Message>, sys: &System) {
    let mut usages: Vec<f32> = Vec::new();
    for cpu in sys.cpus().iter() {
        usages.push(cpu.cpu_usage());
    }
    tx.send(Message::CPUUsage(usages)).unwrap();
}

pub fn seconds_to_timestamp(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let days = hours /  24;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{} days {}:{}:{}", days, hours, minutes, seconds)
}