#[derive (Default)]
pub struct Disk {
    pub name: String,
    pub file_system: String,
    pub total_space: u64,
    pub available_space: u64,
}

impl Disk {
    pub fn new(name: String, file_system: String, total_space: u64, available_space: u64) -> Self {
        Self {
            name,
            file_system,
            total_space,
            available_space,
        }
    }
    
    pub fn percent_used_space(&self) -> u64 {
        let used_space = self.total_space - self.available_space;
        return used_space * 100 / self.total_space;
    }
}
