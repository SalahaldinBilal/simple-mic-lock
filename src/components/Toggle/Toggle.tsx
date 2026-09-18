import styles from "./Toggle.module.scss";

type ToggleProps = {
  checked: boolean,
  onChange: (checked: boolean) => void,
  label: string,
};

function Toggle(props: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={props.checked}
      aria-label={props.label}
      class={styles.Toggle}
      classList={{ [styles.On]: props.checked }}
      onClick={() => props.onChange(!props.checked)}
    >
      <span class={styles.Knob} />
    </button>
  );
}

export default Toggle;
