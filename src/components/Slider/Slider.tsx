import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import styles from "./Slider.module.scss";

type SliderProps = {
  value: number,
  marker: number | null,
  active: boolean,
  disabled: boolean,
  label: string,
  onInput: (value: number) => void,
};

const SETTLE_TIMEOUT = 600;

function Slider(props: SliderProps) {
  const [draft, setDraft] = createSignal<number | null>(null);
  const [dragging, setDragging] = createSignal(false);
  const shown = () => draft() ?? props.value;
  const showMarker = () => props.marker !== null && Math.abs(props.marker - shown()) >= 1;

  let track!: HTMLDivElement;
  let settleTimer: ReturnType<typeof setTimeout> | undefined;

  createEffect(() => {
    if (!dragging() && draft() === props.value) setDraft(null);
  });

  onCleanup(() => clearTimeout(settleTimer));

  const change = (value: number) => {
    const next = Math.min(100, Math.max(0, Math.round(value)));
    if (next === shown()) return;
    setDraft(next);
    props.onInput(next);
  };

  const settle = () => {
    clearTimeout(settleTimer);
    settleTimer = setTimeout(() => setDraft(null), SETTLE_TIMEOUT);
  };

  const valueAt = (clientX: number) => {
    const rect = track.getBoundingClientRect();
    return ((clientX - rect.left) / rect.width) * 100;
  };

  const onPointerDown = (event: PointerEvent) => {
    if (props.disabled || event.button !== 0) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    clearTimeout(settleTimer);
    setDragging(true);
    change(valueAt(event.clientX));
  };

  const onPointerMove = (event: PointerEvent) => {
    if (dragging()) change(valueAt(event.clientX));
  };

  const onPointerUp = () => {
    if (!dragging()) return;
    setDragging(false);
    settle();
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (props.disabled) return;
    const steps: Record<string, number> = {
      ArrowLeft: -1, ArrowDown: -1, ArrowRight: 1, ArrowUp: 1, PageDown: -10, PageUp: 10,
    };
    if (event.key in steps) change(shown() + steps[event.key]);
    else if (event.key === "Home") change(0);
    else if (event.key === "End") change(100);
    else return;
    event.preventDefault();
    settle();
  };

  return (
    <div class={styles.Row}>
      <div
        class={styles.Slider}
        classList={{ [styles.Active]: props.active, [styles.Disabled]: props.disabled, [styles.Dragging]: dragging() }}
        role="slider"
        tabIndex={props.disabled ? -1 : 0}
        aria-label={props.label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={shown()}
        aria-disabled={props.disabled}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onKeyDown={onKeyDown}
      >
        <div class={styles.Track} ref={track}>
          <div class={styles.Fill} style={{ width: `${shown()}%` }} />
          <Show when={showMarker()}>
            <div class={styles.Marker} style={{ left: `${props.marker}%` }} />
          </Show>
          <div class={styles.Knob} style={{ left: `${shown()}%` }} />
        </div>
      </div>
      <span class={styles.Value}>{shown()}%</span>
    </div>
  );
}

export default Slider;
