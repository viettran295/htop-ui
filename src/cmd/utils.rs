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