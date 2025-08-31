#[derive(Default, Clone)]
pub struct Temperature {
    pub label: String,
    pub value: f32,
    pub max: f32,
    pub critical: f32,
}

impl Temperature {
    pub fn new(label: String, value: f32, max: f32, critical: f32) -> Self {
        Self {
            label,
            value,
            max,
            critical
        }
    }
}
