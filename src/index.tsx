/* @refresh reload */
import { render } from "solid-js/web";
import '@fontsource-variable/inter/index.css';
import './App.scss';
import Main from "./pages/Main/Main";

if (!import.meta.env.DEV) {
  document.addEventListener("contextmenu", event => {
    if (!(event.target instanceof HTMLInputElement)) event.preventDefault();
  });
}

render(() => <Main />, document.getElementById("root") as HTMLElement);
