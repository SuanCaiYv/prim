import {createApp} from 'vue'
import './style.css'
import App from './App.vue'
import router from './router/index'
import {appWindow} from "@tauri-apps/api/window";
import {Cmd, Head, Msg, Type} from "./api/backend/entity";
import {timestamp} from "./util/base";

createApp(App).use(router).mount('#app')

// const msg = new Msg(new Head(4, Type.Text, 1, 2, timestamp(), 0, 0), "qwer");
// const cmd = new Cmd("test", Array.from([new TextEncoder().encode("abcd")]));
// const a = appWindow.emit("connect", "127.0.0.1:8190");
// const b = appWindow.listen("cmd-res", event => {
//     console.log(Cmd.fromObj(event.payload))
// })
// setInterval(() => {
//     const c = appWindow.emit("cmd", new Cmd("send-msg", Array.from([msg.toUint8Array()])).toObj());
// }, 3000)
