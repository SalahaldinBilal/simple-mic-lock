import { createMemo, createRoot } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { listen } from "@tauri-apps/api/event";
import { DevicesSnapshot } from "@core/types";
import { safeInvoke } from "@core/helpers/safeInvoke";

function useDevicesStateInner() {
  const [snapshot, setSnapshot] = createStore<DevicesSnapshot>({ devices: [], unidentified: [] });

  const apply = (next: DevicesSnapshot) => setSnapshot(reconcile(next, { key: "key", merge: true }));

  listen<DevicesSnapshot>("devices://changed", event => apply(event.payload))
    .then(() => safeInvoke("list_devices"))
    .then(apply);

  const connected = createMemo(() => snapshot.devices.filter(device => device.connected).length + snapshot.unidentified.length);
  const locked = createMemo(() => snapshot.devices.filter(device => device.connected && device.locked).length);

  return {
    snapshot,
    connected,
    locked,
    setTarget: (key: string, percent: number) => safeInvoke("set_target", { key, percent }),
    setLocked: (key: string, locked: boolean) => safeInvoke("set_locked", { key, locked }),
    setNickname: (key: string, nickname: string | null) => safeInvoke("set_nickname", { key, nickname }),
    claim: (endpoint: string, key: string | null) => safeInvoke("claim_device", { endpoint, key }),
    forget: (key: string) => safeInvoke("forget_device", { key }),
  };
}

const useDevicesState = createRoot(useDevicesStateInner);
export default useDevicesState;
