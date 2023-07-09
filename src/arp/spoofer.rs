pub trait ArpSpoofer {
    fn startForIp(&self, ip: &str);
}

pub struct ArpSpooferImpl {
    spoofed: String
}

impl ArpSpooferImpl {
    pub fn new(spoofed: &str) -> Self {
        Self {
            spoofed: spoofed.to_string()
        }
    }
}

impl ArpSpoofer for ArpSpooferImpl {
    fn startForIp(&self, target: &str) {
        // no-op
    }
}