#![allow(missing_docs)]

use inazuma::{Oklch, oklch, oklcha};

#[derive(Clone, Debug, PartialEq)]
pub struct SystemColors {
    pub transparent: Oklch,
    pub mac_os_traffic_light_red: Oklch,
    pub mac_os_traffic_light_yellow: Oklch,
    pub mac_os_traffic_light_green: Oklch,
}

impl Default for SystemColors {
    fn default() -> Self {
        Self {
            transparent: oklcha(0.0, 0.0, 0.0, 0.0),
            mac_os_traffic_light_red: oklch(0.6824, 0.1623, 27.3944),
            mac_os_traffic_light_yellow: oklch(0.8323, 0.1411, 83.5801),
            mac_os_traffic_light_green: oklch(0.7372, 0.1774, 141.3034),
        }
    }
}
