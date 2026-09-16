use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_Device_IDW, CM_Get_Parent, CM_LOCATE_DEVNODE_NORMAL, CM_Locate_DevNodeW, CR_SUCCESS,
    MAX_DEVICE_ID_LEN,
};
use windows::core::PCWSTR;

use crate::state::Hardware;
use crate::wide::wide;

/// Endpoints with no USB device above them in the device tree keep their Windows endpoint ID.
pub fn hardware(endpoint: &str) -> Hardware {
    usb_ancestor(endpoint).unwrap_or(Hardware::Endpoint)
}

fn usb_ancestor(endpoint: &str) -> Option<Hardware> {
    let path = wide(&format!(r"SWD\MMDEVAPI\{endpoint}"));
    let mut node = 0u32;
    unsafe {
        if CM_Locate_DevNodeW(&mut node, PCWSTR(path.as_ptr()), CM_LOCATE_DEVNODE_NORMAL)
            != CR_SUCCESS
        {
            return None;
        }

        loop {
            let mut parent = 0u32;
            if CM_Get_Parent(&mut parent, node, 0) != CR_SUCCESS {
                return None;
            }
            node = parent;
            if let Some(hardware) = instance_id(node).as_deref().and_then(parse_usb) {
                return Some(hardware);
            }
        }
    }
}

fn instance_id(node: u32) -> Option<String> {
    let mut buffer = [0u16; MAX_DEVICE_ID_LEN as usize + 1];
    if unsafe { CM_Get_Device_IDW(node, &mut buffer, 0) } != CR_SUCCESS {
        return None;
    }
    let length = buffer.iter().position(|&unit| unit == 0)?;
    Some(String::from_utf16_lossy(&buffer[..length]))
}

fn parse_usb(instance: &str) -> Option<Hardware> {
    let instance = instance.to_uppercase();
    let mut parts = instance.split('\\');
    if parts.next()? != "USB" {
        return None;
    }
    let hardware = parts.next()?;
    let unit = parts.next()?;
    if hardware.contains("&MI_") {
        return None;
    }

    let model = format!("usb:{}:{}", field(hardware, "VID_")?, field(hardware, "PID_")?);

    // Windows only generates an instance ID containing '&' when the device reports no serial number.
    Some(if unit.contains('&') {
        Hardware::Anonymous { model, port: unit.to_owned() }
    } else {
        Hardware::Serial { model, serial: unit.to_owned() }
    })
}

fn field<'a>(hardware: &'a str, prefix: &str) -> Option<&'a str> {
    hardware.split('&').find_map(|part| part.strip_prefix(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_device_without_serial_is_keyed_by_model_with_its_port() {
        assert_eq!(
            parse_usb(r"USB\VID_0D8C&PID_016C\6&c1a2e2f&0&1"),
            Some(Hardware::Anonymous {
                model: "usb:0D8C:016C".into(),
                port: "6&C1A2E2F&0&1".into(),
            })
        );
    }

    #[test]
    fn usb_device_with_serial_is_keyed_by_serial() {
        assert_eq!(
            parse_usb(r"USB\VID_046D&PID_0A44\ABC123"),
            Some(Hardware::Serial { model: "usb:046D:0A44".into(), serial: "ABC123".into() })
        );
    }

    #[test]
    fn interfaces_hubs_and_other_buses_are_not_the_usb_device() {
        assert_eq!(parse_usb(r"USB\VID_0D8C&PID_016C&MI_00\7&1029c449&0&0000"), None);
        assert_eq!(parse_usb(r"USB\ROOT_HUB30\5&2c35141&0&0"), None);
        assert_eq!(parse_usb(r"PCI\VEN_1022&DEV_149C&SUBSYS_87C01043&REV_00\4&231a312e&0&0341"), None);
    }
}
