import { JSX, splitProps } from "solid-js";
import styles from "./Button.module.scss";

type ButtonProps = JSX.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "accent" | "quiet" | "outline",
};

function Button(props: ButtonProps) {
  const [local, rest] = splitProps(props, ["variant", "class"]);

  return (
    <button
      type="button"
      {...rest}
      class={`${styles.Button} ${styles[local.variant ?? "outline"]} ${local.class ?? ""}`}
    />
  );
}

export default Button;
