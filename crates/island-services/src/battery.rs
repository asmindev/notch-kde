use futures_lite::StreamExt;
use island_core::models::BatteryState;
use std::fs;
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};
use zbus::MatchRule;
use zbus::MessageStream;

pub struct BatteryService;

impl BatteryService {
    /// Reads battery status and AC connection directly from sysfs with <1ms latency.
    pub fn read_battery() -> BatteryState {
        // 1. Check AC / Mains power supply adapter
        let mut ac_online = false;
        if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
            for entry in entries.flatten() {
                let path = entry.path();
                let type_path = path.join("type");
                if let Ok(supply_type) = fs::read_to_string(&type_path) {
                    let st = supply_type.trim();
                    if st.eq_ignore_ascii_case("mains") || st.eq_ignore_ascii_case("ac") {
                        if let Ok(online_val) = fs::read_to_string(path.join("online")) {
                            if online_val.trim() == "1" {
                                ac_online = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // 2. Fallback check for common AC nodes if dir iteration was empty
        if !ac_online {
            for adp in ["/sys/class/power_supply/ADP1/online", "/sys/class/power_supply/AC/online", "/sys/class/power_supply/ACAD/online"] {
                if let Ok(val) = fs::read_to_string(adp) {
                    if val.trim() == "1" {
                        ac_online = true;
                        break;
                    }
                }
            }
        }

        // 3. Check Battery nodes
        let bat_dir = Path::new("/sys/class/power_supply/BAT0");
        let alt_bat = Path::new("/sys/class/power_supply/BAT1");

        let target_dir = if bat_dir.exists() {
            bat_dir
        } else if alt_bat.exists() {
            alt_bat
        } else {
            // Desktop or machine without onboard battery
            return BatteryState {
                percentage: 100,
                is_charging: true,
            };
        };

        let capacity = fs::read_to_string(target_dir.join("capacity"))
            .unwrap_or_default()
            .trim()
            .parse::<u8>()
            .unwrap_or(100);

        let status = fs::read_to_string(target_dir.join("status"))
            .unwrap_or_default()
            .trim()
            .to_lowercase();

        // Charging is true when AC is online or battery reports charging/full
        let is_charging = ac_online
            || status == "charging"
            || status == "full"
            || (ac_online && status == "not charging");

        BatteryState {
            percentage: capacity,
            is_charging,
        }
    }

    pub fn start(sender: UnboundedSender<BatteryState>) {
        let sender_clone = sender.clone();

        // 1. Instant Kernel Netlink UEvent Socket (< 1ms reaction to AC plug/unplug, matching pill-panel)
        std::thread::spawn(move || {
            unsafe {
                let fd = libc::socket(libc::AF_NETLINK, libc::SOCK_RAW, libc::NETLINK_KOBJECT_UEVENT);
                if fd >= 0 {
                    let mut sa: libc::sockaddr_nl = std::mem::zeroed();
                    sa.nl_family = libc::AF_NETLINK as u16;
                    sa.nl_groups = 1; // multicast group 1 for kernel uevents

                    if libc::bind(
                        fd,
                        &sa as *const libc::sockaddr_nl as *const libc::sockaddr,
                        std::mem::size_of::<libc::sockaddr_nl>() as u32,
                    ) == 0 {
                        info!("Kernel Netlink UEvent socket active for instant power events");
                        let mut buf = [0u8; 2048];
                        let mut last_state = BatteryService::read_battery();

                        loop {
                            let len = libc::recv(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len() - 1, 0);
                            if len <= 0 {
                                break;
                            }
                            let msg = String::from_utf8_lossy(&buf[..len as usize]);
                            if msg.contains("power_supply")
                                || msg.contains("ADP")
                                || msg.contains("BAT")
                                || msg.contains("AC")
                            {
                                let state = BatteryService::read_battery();
                                if state != last_state {
                                    debug!(
                                        "Instant Netlink power event: {}% (charging: {})",
                                        state.percentage, state.is_charging
                                    );
                                    last_state = state.clone();
                                    if sender_clone.send(state).is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    libc::close(fd);
                }
            }
        });

        // 2. UPower D-Bus Wildcard Listener (System Bus events)
        tokio::spawn(async move {
            info!("Event-driven Battery service starting via UPower D-Bus system bus");
            let conn = match zbus::Connection::system().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to connect to system D-Bus for battery events: {}", e);
                    return;
                }
            };

            let mut last_state = Self::read_battery();
            let _ = sender.send(last_state.clone());

            // Match rule for PropertiesChanged signals from UPower
            let prop_rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .interface("org.freedesktop.DBus.Properties")
                .and_then(|b| b.member("PropertiesChanged"))
                .and_then(|b| b.sender("org.freedesktop.UPower"))
                .map(|b| b.build());

            let mut stream = match prop_rule {
                Ok(rule) => match MessageStream::for_match_rule(rule, &conn, None).await {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to subscribe to UPower signal match rule: {}", e);
                        return;
                    }
                },
                Err(e) => {
                    error!("Failed to build UPower match rule: {}", e);
                    return;
                }
            };

            info!("Battery service subscribed to UPower D-Bus signals (zero polling)");

            while let Some(_msg) = stream.next().await {
                let state = Self::read_battery();
                if state != last_state {
                    debug!(
                        "Battery updated via UPower event: {}% (charging: {})",
                        state.percentage, state.is_charging
                    );
                    last_state = state.clone();
                    if sender.send(state).is_err() {
                        break;
                    }
                }
            }
            info!("Battery event stream ended");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_read() {
        let b = BatteryService::read_battery();
        println!(
            "Battery read: percentage={}, is_charging={}",
            b.percentage, b.is_charging
        );
        assert!(b.percentage <= 100);
    }
}
