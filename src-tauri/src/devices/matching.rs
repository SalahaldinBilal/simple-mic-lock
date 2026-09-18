use std::collections::BTreeMap;

use super::{Candidate, Device, Discovered, Hardware, Prompt, Unidentified};

pub fn apply(devices: &mut Vec<Device>, discovered: Vec<Discovered>) -> Vec<Unidentified> {
    for device in devices.iter_mut() {
        device.endpoint = None;
        device.adjustable = false;
        device.is_default = false;
        device.level = None;
    }

    let mut anonymous: BTreeMap<String, Vec<Discovered>> = BTreeMap::new();
    for unit in discovered {
        let key = match &unit.hardware {
            Hardware::Endpoint => unit.endpoint.clone(),
            Hardware::Serial { model, serial } => format!("{model}:{serial}"),
            Hardware::Anonymous { model, .. } => {
                anonymous.entry(model.clone()).or_default().push(unit);
                continue;
            }
        };

        let taken = devices.iter().any(|device| device.key == key && device.present());
        let key = if taken { unit.endpoint.clone() } else { key };

        match devices.iter_mut().find(|device| device.key == key) {
            Some(device) => attach(device, unit),
            None => devices.push(create(key, unit)),
        }
    }

    let mut unidentified = Vec::new();
    for (model, units) in anonymous {
        unidentified.extend(resolve(devices, &model, units));
    }
    unidentified
}

pub fn claim(
    devices: &mut Vec<Device>,
    unidentified: &mut Vec<Unidentified>,
    endpoint: &str,
    choice: Option<&str>,
) {
    let Some(index) = unidentified.iter().position(|unit| unit.discovered.endpoint == endpoint)
    else {
        return;
    };
    let Hardware::Anonymous { model, .. } = unidentified[index].discovered.hardware.clone() else {
        return;
    };

    match choice {
        Some(key) => {
            let Some(device) = devices.iter_mut().find(|device| {
                device.key == key && !device.present() && device.model.as_deref() == Some(&model)
            }) else {
                return;
            };
            attach(device, unidentified.remove(index).discovered);
        }
        None => {
            let unit = unidentified.remove(index).discovered;
            devices.push(create(String::new(), unit));
        }
    }
    rekey(devices, &model);

    let (same_model, others): (Vec<_>, Vec<_>) = unidentified.drain(..).partition(|unit| {
        matches!(&unit.discovered.hardware, Hardware::Anonymous { model: other, .. } if *other == model)
    });
    *unidentified = others;
    unidentified.extend(resolve(
        devices,
        &model,
        same_model.into_iter().map(|unit| unit.discovered).collect(),
    ));
}

fn resolve(devices: &mut Vec<Device>, model: &str, units: Vec<Discovered>) -> Vec<Unidentified> {
    let mut waiting = Vec::new();
    for unit in units {
        let Hardware::Anonymous { port, .. } = &unit.hardware else { continue };
        let same_port = devices.iter_mut().find(|device| {
            !device.present()
                && device.model.as_deref() == Some(model)
                && device.port.as_ref() == Some(port)
        });
        match same_port {
            Some(device) => attach(device, unit),
            None => waiting.push(unit),
        }
    }

    let saved: Vec<usize> = devices
        .iter()
        .enumerate()
        .filter(|(_, device)| !device.present() && device.model.as_deref() == Some(model))
        .map(|(index, _)| index)
        .collect();

    let mut unidentified = Vec::new();
    match (waiting.len(), saved.len()) {
        (0, _) => {}
        (1, 1) => attach(&mut devices[saved[0]], waiting.remove(0)),
        (_, 0) => devices.extend(waiting.into_iter().map(|unit| create(String::new(), unit))),
        (1, _) => {
            let candidates = saved
                .iter()
                .map(|&index| Candidate {
                    key: devices[index].key.clone(),
                    name: devices[index].display_name().to_owned(),
                })
                .collect();
            unidentified.push(Unidentified {
                discovered: waiting.remove(0),
                prompt: Prompt::Choose(candidates),
            });
        }
        (_, count) => {
            let keep = (count == 1).then(|| devices[saved[0]].display_name().to_owned());
            unidentified.extend(waiting.into_iter().map(|unit| Unidentified {
                discovered: unit,
                prompt: Prompt::KeepOnlyOne(keep.clone()),
            }));
        }
    }

    rekey(devices, model);
    unidentified
}

fn rekey(devices: &mut [Device], model: &str) {
    let members: Vec<usize> = devices
        .iter()
        .enumerate()
        .filter(|(_, device)| device.model.as_deref() == Some(model))
        .map(|(index, _)| index)
        .collect();

    if let [only] = members[..] {
        if devices[only].key.is_empty() {
            devices[only].key = model.to_owned();
        }
        return;
    }

    for index in members {
        if devices[index].key.is_empty() || devices[index].key == model {
            devices[index].key = next_unit_key(devices, model);
        }
    }
}

fn next_unit_key(devices: &[Device], model: &str) -> String {
    (1..)
        .map(|unit| format!("{model}#{unit}"))
        .find(|key| devices.iter().all(|device| &device.key != key))
        .expect("unit numbers are unbounded")
}

fn create(key: String, unit: Discovered) -> Device {
    let mut device = Device {
        key,
        name: unit.endpoint.clone(),
        nickname: None,
        target: unit.level.map_or(100, |level| (level * 100.0).round() as u32),
        locked: false,
        model: None,
        port: None,
        endpoint: None,
        adjustable: false,
        is_default: false,
        level: None,
    };
    attach(&mut device, unit);
    device
}

fn attach(device: &mut Device, unit: Discovered) {
    if let Some(name) = unit.name {
        device.name = name;
    }
    if let Hardware::Anonymous { model, port } = unit.hardware {
        device.model = Some(model);
        device.port = Some(port);
    }
    device.endpoint = Some(unit.endpoint);
    device.adjustable = unit.adjustable;
    device.is_default = unit.is_default;
    device.level = unit.level;
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = "usb:0D8C:016C";

    fn saved(key: &str, port: &str, nickname: &str) -> Device {
        Device {
            key: key.into(),
            name: "Microphone (USB Advanced Audio Device)".into(),
            nickname: Some(nickname.into()),
            target: 70,
            locked: true,
            model: Some(MODEL.into()),
            port: Some(port.into()),
            endpoint: None,
            adjustable: false,
            is_default: false,
            level: None,
        }
    }

    fn plugged(endpoint: &str, port: &str) -> Discovered {
        Discovered {
            endpoint: endpoint.into(),
            hardware: Hardware::Anonymous { model: MODEL.into(), port: port.into() },
            name: Some("Microphone (USB Advanced Audio Device)".into()),
            adjustable: true,
            is_default: false,
            level: Some(0.5),
        }
    }

    fn on<'a>(devices: &'a [Device], endpoint: &str) -> &'a Device {
        devices
            .iter()
            .find(|device| device.endpoint.as_deref() == Some(endpoint))
            .expect("endpoint is attached to a device")
    }

    #[test]
    fn a_single_mic_keeps_its_settings_on_a_new_port() {
        let mut devices = vec![saved(MODEL, "P0", "Desk")];
        let unidentified = apply(&mut devices, vec![plugged("e1", "P1")]);

        assert!(unidentified.is_empty());
        assert_eq!(devices.len(), 1);
        let desk = on(&devices, "e1");
        assert_eq!(desk.key, MODEL);
        assert!(desk.locked);
        assert_eq!(desk.port.as_deref(), Some("P1"));
    }

    #[test]
    fn a_new_mic_is_keyed_by_model_only() {
        let mut devices = Vec::new();
        apply(&mut devices, vec![plugged("e1", "P1")]);
        assert_eq!(devices[0].key, MODEL);
        assert!(!devices[0].locked);
        assert_eq!(devices[0].target, 50);
    }

    #[test]
    fn a_second_identical_mic_gets_its_own_settings() {
        let mut devices = vec![saved(MODEL, "P0", "Desk")];
        let unidentified = apply(&mut devices, vec![plugged("e1", "P0"), plugged("e2", "P1")]);

        assert!(unidentified.is_empty());
        let desk = on(&devices, "e1");
        let second = on(&devices, "e2");
        assert_eq!(desk.nickname.as_deref(), Some("Desk"));
        assert!(desk.locked);
        assert!(!second.locked);
        assert_ne!(desk.key, second.key);
        assert!(desk.key.starts_with(&format!("{MODEL}#")));
        assert!(second.key.starts_with(&format!("{MODEL}#")));
    }

    #[test]
    fn identical_mics_on_unknown_ports_are_never_guessed() {
        let mut devices = vec![saved(MODEL, "P0", "Desk")];
        let unidentified = apply(&mut devices, vec![plugged("e1", "P1"), plugged("e2", "P2")]);

        assert_eq!(unidentified.len(), 2);
        assert!(
            unidentified
                .iter()
                .all(|unit| unit.prompt == Prompt::KeepOnlyOne(Some("Desk".into())))
        );
        assert!(!devices[0].present());
    }

    #[test]
    fn keeping_only_the_saved_mic_plugged_in_resolves_it() {
        let mut devices = vec![saved(MODEL, "P0", "Desk")];
        apply(&mut devices, vec![plugged("e1", "P1"), plugged("e2", "P2")]);
        let unidentified = apply(&mut devices, vec![plugged("e1", "P1")]);

        assert!(unidentified.is_empty());
        assert_eq!(on(&devices, "e1").nickname.as_deref(), Some("Desk"));
    }

    #[test]
    fn known_identical_mics_are_matched_by_port() {
        let left = format!("{MODEL}#1");
        let right = format!("{MODEL}#2");
        let mut devices = vec![saved(&left, "P1", "Left"), saved(&right, "P2", "Right")];
        let unidentified = apply(&mut devices, vec![plugged("e1", "P2"), plugged("e2", "P1")]);

        assert!(unidentified.is_empty());
        assert_eq!(on(&devices, "e1").key, right);
        assert_eq!(on(&devices, "e2").key, left);
    }

    #[test]
    fn one_mic_on_a_new_port_with_several_saved_asks_which_it_is() {
        let mut devices = vec![
            saved(&format!("{MODEL}#1"), "P1", "Left"),
            saved(&format!("{MODEL}#2"), "P2", "Right"),
        ];
        let unidentified = apply(&mut devices, vec![plugged("e3", "P3")]);

        assert_eq!(unidentified.len(), 1);
        let Prompt::Choose(candidates) = &unidentified[0].prompt else {
            panic!("expected a choice between saved mics");
        };
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn choosing_a_saved_mic_moves_its_settings_to_the_new_port() {
        let right = format!("{MODEL}#2");
        let mut devices = vec![saved(&format!("{MODEL}#1"), "P1", "Left"), saved(&right, "P2", "Right")];
        let mut unidentified = apply(&mut devices, vec![plugged("e3", "P3")]);
        claim(&mut devices, &mut unidentified, "e3", Some(&right));

        assert!(unidentified.is_empty());
        let chosen = on(&devices, "e3");
        assert_eq!(chosen.nickname.as_deref(), Some("Right"));
        assert_eq!(chosen.port.as_deref(), Some("P3"));
    }

    #[test]
    fn choosing_new_mic_creates_a_separate_entry() {
        let mut devices = vec![
            saved(&format!("{MODEL}#1"), "P1", "Left"),
            saved(&format!("{MODEL}#2"), "P2", "Right"),
        ];
        let mut unidentified = apply(&mut devices, vec![plugged("e3", "P3")]);
        claim(&mut devices, &mut unidentified, "e3", None);

        assert!(unidentified.is_empty());
        assert_eq!(devices.len(), 3);
        assert_eq!(on(&devices, "e3").key, format!("{MODEL}#3"));
        assert!(on(&devices, "e3").nickname.is_none());
    }

    #[test]
    fn mics_with_serial_numbers_are_keyed_by_serial() {
        let mut devices = Vec::new();
        let unit = Discovered {
            endpoint: "e1".into(),
            hardware: Hardware::Serial { model: MODEL.into(), serial: "SN1".into() },
            name: None,
            adjustable: true,
            is_default: false,
            level: Some(0.3),
        };
        apply(&mut devices, vec![unit]);

        assert_eq!(devices[0].key, format!("{MODEL}:SN1"));
        assert_eq!(devices[0].target, 30);
    }
}
