import { For, Match, Switch } from "solid-js";
import { UnidentifiedDevice } from "@core/types";
import Button from "@core/components/Button/Button";
import useDevicesState from "@core/states/devicesState";
import cardStyles from "../DeviceCard/DeviceCard.module.scss";
import styles from "./UnidentifiedCard.module.scss";

type UnidentifiedCardProps = {
  unit: UnidentifiedDevice,
};

function UnidentifiedCard(props: UnidentifiedCardProps) {
  const { claim } = useDevicesState;

  return (
    <div class={`${cardStyles.Card} ${styles.Unidentified}`}>
      <div class={cardStyles.Top}>
        <span class={cardStyles.Name} title={props.unit.name}>{props.unit.name}</span>
      </div>
      <div class={styles.Warning}>Not identified yet</div>

      <Switch>
        <Match when={props.unit.prompt.kind === "choose" && props.unit.prompt}>
          {prompt => (
            <div class={styles.Prompt}>
              <span>Which mic is this?</span>
              <div class={styles.Choices}>
                <For each={prompt().candidates}>
                  {candidate => (
                    <Button onClick={() => claim(props.unit.endpoint, candidate.key)}>{candidate.name}</Button>
                  )}
                </For>
                <Button variant="quiet" onClick={() => claim(props.unit.endpoint, null)}>New mic</Button>
              </div>
            </div>
          )}
        </Match>
        <Match when={props.unit.prompt.kind === "keepOnlyOne" && props.unit.prompt}>
          {prompt => (
            <p class={styles.Prompt}>
              {prompt().keep
                ? `Several identical mics are plugged in. Keep only "${prompt().keep}" connected, then plug the others back in one at a time.`
                : "Several identical mics are plugged in. Keep only one of them connected, then plug the others back in one at a time."}
            </p>
          )}
        </Match>
      </Switch>
    </div>
  );
}

export default UnidentifiedCard;
