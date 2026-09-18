import { JSX, splitProps } from "solid-js";
import styles from "./IconButton.module.scss";

type IconButtonProps = JSX.ButtonHTMLAttributes<HTMLButtonElement> & {
  active?: boolean,
};

function IconButton(props: IconButtonProps) {
  const [local, rest] = splitProps(props, ["active"]);

  return (
    <button
      type="button"
      {...rest}
      class={styles.IconButton}
      classList={{ [styles.Active]: local.active ?? false }}
    />
  );
}

export default IconButton;
