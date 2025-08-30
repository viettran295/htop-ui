#[derive(Debug, Clone, Copy)]
pub struct Network {
    pub upload: f64,
    pub download: f64,
}

impl Network {
    pub fn new() -> Self {
        Self { 
            upload: 0.0, 
            download: 0.0
        }
    }
    
    pub fn update(&mut self, upload: f64, download: f64) {
        self.upload = upload;
        self.download = download;
    }
}