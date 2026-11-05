import { mount } from "svelte";
import App from "./App.svelte";
import { applyTheme } from "./lib/theme/apply";
import "./app.css";

applyTheme("system");

const app = mount(App, { target: document.getElementById("app")! });

export default app;
