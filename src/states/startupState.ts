import { createRoot, createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { safeInvoke } from "@core/helpers/safeInvoke";

function useStartupStateInner() {
  const [autostart, setAutostartSignal] = createSignal(false);
  const [showTray, setShowTraySignal] = createSignal(true);

  listen<boolean>("autostart://changed", event => setAutostartSignal(event.payload))
    .then(() => safeInvoke("get_autostart"))
    .then(setAutostartSignal);

  listen<boolean>("tray://changed", event => setShowTraySignal(event.payload))
    .then(() => safeInvoke("get_show_tray"))
    .then(setShowTraySignal);

  return {
    autostart,
    setAutostart: (enabled: boolean) => safeInvoke("set_autostart", { enabled }),
    showTray,
    setShowTray: (enabled: boolean) => safeInvoke("set_show_tray", { enabled }),
  };
}

const useStartupState = createRoot(useStartupStateInner);
export default useStartupState;
