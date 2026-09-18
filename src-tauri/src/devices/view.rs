use serde::Serialize;

use super::{Device, Prompt, Unidentified};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    devices: Vec<DeviceView>,
    unidentified: Vec<UnidentifiedView>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DeviceView {
    key: String,
    name: String,
    nickname: Option<String>,
    target: u32,
    locked: bool,
    connected: bool,
    adjustable: bool,
    is_default: bool,
    level: Option<u32>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UnidentifiedView {
    endpoint: String,
    name: String,
    level: Option<u32>,
    prompt: PromptView,
}

#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum PromptView {
    Choose { candidates: Vec<CandidateView> },
    KeepOnlyOne { keep: Option<String> },
}

#[derive(Serialize, Clone)]
struct CandidateView {
    key: String,
    name: String,
}

impl Snapshot {
    pub fn new(devices: &[Device], unidentified: &[Unidentified]) -> Self {
        Self {
            devices: devices
                .iter()
                .map(|device| DeviceView {
                    key: device.key.clone(),
                    name: device.name.clone(),
                    nickname: device.nickname.clone(),
                    target: device.target,
                    locked: device.locked,
                    connected: device.present(),
                    adjustable: device.adjustable,
                    is_default: device.is_default,
                    level: device.level.map(percent),
                })
                .collect(),
            unidentified: unidentified
                .iter()
                .map(|unit| UnidentifiedView {
                    endpoint: unit.discovered.endpoint.clone(),
                    name: unit.discovered.name.clone().unwrap_or_else(|| "Microphone".into()),
                    level: unit.discovered.level.map(percent),
                    prompt: match &unit.prompt {
                        Prompt::Choose(candidates) => PromptView::Choose {
                            candidates: candidates
                                .iter()
                                .map(|candidate| CandidateView {
                                    key: candidate.key.clone(),
                                    name: candidate.name.clone(),
                                })
                                .collect(),
                        },
                        Prompt::KeepOnlyOne(keep) => PromptView::KeepOnlyOne { keep: keep.clone() },
                    },
                })
                .collect(),
        }
    }
}

fn percent(level: f32) -> u32 {
    (level * 100.0).round().clamp(0.0, 100.0) as u32
}
