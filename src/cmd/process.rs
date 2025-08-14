#[derive(Debug, Default, Clone)]
pub struct Process {
    pub pid: u32,
    pub process_name: String,
    pub user: String,
    pub cpu_usage: f32,
    pub mem_usage: f32,
}

impl Process {    
    pub fn set_pid(mut self, pid: u32) -> Self {
        self.pid = pid;
        self
    }
    
    pub fn set_process_name(mut self, process_name: String) -> Self {
        self.process_name = process_name;
        self
    }
    
    pub fn set_user(mut self, user: String) -> Self {
        self.user = user;
        self
    }
    
    pub fn set_cpu_usage(mut self, cpu_usage: f32) -> Self {
        self.cpu_usage = cpu_usage;
        self
    }
    
    pub fn set_mem_usage(mut self, mem_usage: f32) -> Self {
        self.mem_usage = mem_usage.round();
        self
    }
    
    pub fn build(self) -> Result<Process, ()> {
        Ok(Process {
            pid: self.pid,
            process_name: self.process_name,
            user: self.user,
            cpu_usage: self.cpu_usage,
            mem_usage: self.mem_usage
        })
    }
    
    pub fn sort_most_consume_cpu(processes: &mut Vec<Process>) {
        processes.sort_by(|a, b| b.cpu_usage
                            .partial_cmp(&a.cpu_usage)
                            .unwrap_or(std::cmp::Ordering::Equal));
    }
}