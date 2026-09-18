import { createResource, For, Show } from "solid-js";
import { getVersion } from "@tauri-apps/api/app";
import Button from "@core/components/Button/Button";
import Toggle from "@core/components/Toggle/Toggle";
import useDevicesState from "@core/states/devicesState";
import useStartupState from "@core/states/startupState";
import { safeInvoke } from "@core/helpers/safeInvoke";
import DeviceCard from "./DeviceCard/DeviceCard";
import UnidentifiedCard from "./UnidentifiedCard/UnidentifiedCard";
import styles from "./Main.module.scss";

function Main() {
  const { snapshot, connected, locked } = useDevicesState;
  const { autostart, setAutostart, showTray, setShowTray } = useStartupState;
  const [version] = createResource(getVersion);

  const status = () => {
    if (connected() === 0) return "NO MIC";
    if (locked() === 0) return "NOTHING LOCKED";
    if (locked() === connected()) return "LOCKED";
    return `${locked()} OF ${connected()} LOCKED`;
  };

  const isEmpty = () => snapshot.devices.length === 0 && snapshot.unidentified.length === 0;

  return (
    <div class={styles.Main}>
      <header class={styles.Header}>
        <div class={styles.Title}>
          <span class={styles.AppName}>Simple Mic Lock</span>
          <Show when={version()}>
            <span class={styles.Version}>v{version()}</span>
          </Show>
        </div>
        <span class={styles.Pill} classList={{ [styles.PillActive]: locked() > 0 }}>{status()}</span>
      </header>

      <section class={styles.StartupCard}>
        <div class={styles.StartupRow}>
          <div class={styles.StartupText}>
            <span class={styles.StartupTitle}>Start with Windows</span>
            <span class={styles.StartupHint}>Starts in the background when you sign in</span>
          </div>
          <Toggle label="Start with Windows" checked={autostart()} onChange={setAutostart} />
        </div>
        <div class={styles.StartupRow}>
          <div class={styles.StartupText}>
            <span class={styles.StartupTitle}>Show tray icon</span>
            <span class={styles.StartupHint}>When off, open the app again to get back here</span>
          </div>
          <Toggle label="Show tray icon" checked={showTray()} onChange={setShowTray} />
        </div>
      </section>

      <div class={styles.Caption}>MICROPHONES</div>

      <div class={styles.List}>
        <For each={snapshot.unidentified}>
          {unit => <UnidentifiedCard unit={unit} />}
        </For>
        <For each={snapshot.devices}>
          {device => <DeviceCard device={device} />}
        </For>
        <Show when={isEmpty()}>
          <div class={styles.Empty}>No microphones found. Plug one in and it shows up here.</div>
        </Show>
      </div>

      <footer class={styles.Footer}>
        <Button variant="quiet" onClick={() => safeInvoke("quit")}>Quit</Button>
        <Button variant="accent" onClick={() => safeInvoke("hide_window")}>Hide to tray</Button>
      </footer>
    </div>
  );
}

export default Main;
