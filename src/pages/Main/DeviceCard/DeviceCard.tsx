import { createSignal, Show } from "solid-js";
import LockKeyhole from "lucide-solid/icons/lock-keyhole";
import LockKeyholeOpen from "lucide-solid/icons/lock-keyhole-open";
import Pencil from "lucide-solid/icons/pencil";
import Trash2 from "lucide-solid/icons/trash-2";
import { Device } from "@core/types";
import IconButton from "@core/components/IconButton/IconButton";
import Slider from "@core/components/Slider/Slider";
import useDevicesState from "@core/states/devicesState";
import styles from "./DeviceCard.module.scss";

type DeviceCardProps = {
  device: Device,
};

function DeviceCard(props: DeviceCardProps) {
  const { setTarget, setLocked, setNickname, forget } = useDevicesState;
  const [renaming, setRenaming] = createSignal(false);

  const displayName = () => props.device.nickname ?? props.device.name;
  const controllable = () => !props.device.connected || props.device.adjustable;

  const details = () => [
    props.device.nickname ? props.device.name : null,
    props.device.isDefault ? "Default" : null,
    !props.device.connected
      ? "Not connected"
      : !props.device.adjustable
        ? "No volume control"
        : props.device.level !== null ? `now ${props.device.level}%` : null,
  ].filter(Boolean).join(" · ");

  let cancelled = false;

  const startRename = (input: HTMLInputElement) => {
    cancelled = false;
    requestAnimationFrame(() => {
      input.focus();
      input.select();
    });
  };

  const finishRename = (value: string) => {
    setRenaming(false);
    if (cancelled) return;
    const trimmed = value.trim();
    setNickname(props.device.key, trimmed === "" || trimmed === props.device.name ? null : trimmed);
  };

  return (
    <div class={styles.Card} classList={{ [styles.Disconnected]: !props.device.connected }}>
      <div class={styles.Top}>
        <Show
          when={renaming()}
          fallback={<span class={styles.Name} title={displayName()}>{displayName()}</span>}
        >
          <input
            class={styles.NameInput}
            ref={startRename}
            value={displayName()}
            placeholder={props.device.name}
            spellcheck={false}
            onKeyDown={event => {
              if (event.key === "Enter") event.currentTarget.blur();
              if (event.key === "Escape") {
                cancelled = true;
                event.currentTarget.blur();
              }
            }}
            onBlur={event => finishRename(event.currentTarget.value)}
          />
        </Show>
        <div class={styles.Actions}>
          <Show when={!props.device.connected}>
            <IconButton title="Forget this mic" aria-label="Forget this mic" onClick={() => forget(props.device.key)}>
              <Trash2 size={15} />
            </IconButton>
          </Show>
          <IconButton title="Rename" aria-label="Rename" onClick={() => setRenaming(true)}>
            <Pencil size={15} />
          </IconButton>
          <IconButton
            active={props.device.locked}
            disabled={!controllable()}
            title={props.device.locked ? "Unlock" : "Lock"}
            aria-label={props.device.locked ? "Unlock" : "Lock"}
            aria-pressed={props.device.locked}
            onClick={() => setLocked(props.device.key, !props.device.locked)}
          >
            <Show when={props.device.locked} fallback={<LockKeyholeOpen size={16} />}>
              <LockKeyhole size={16} />
            </Show>
          </IconButton>
        </div>
      </div>

      <Show when={details()}>
        <div class={styles.Details}>{details()}</div>
      </Show>

      <div class={styles.SliderRow}>
        <Slider
          value={props.device.target}
          marker={props.device.connected ? props.device.level : null}
          active={props.device.locked}
          disabled={!controllable()}
          label={`${displayName()} target volume`}
          onInput={percent => setTarget(props.device.key, percent)}
        />
      </div>
    </div>
  );
}

export default DeviceCard;
