import {createApp, reactive} from 'vue'
import './style.css'
import App from './App.vue'
import store from "./store";
import router from "./route";
import {msgChannelMapSynced, userMsgList, userMsgSet} from "./function/types";
import {timestamp} from "./util/base";
import {watch} from "_vue@3.2.37@vue";
import {get} from "_idb-keyval@6.2.0@idb-keyval";
import {Constant} from "./system/constant";
import {hock} from "./function/net";
import {set} from "idb-keyval";
import {Cmd, Msg} from "./api/backend/entity";

createApp(App).use(store).use(router).mount('#app')

// hock.value = false;
//
// let arr = new Array<Map<number, number>>()
// userMsgSet.set(1, new Map<number, number>())
// let i = 0;
// setInterval(() => {
//     if (i >= 6) {
//         return
//     }
//     i ++
//     // @ts-ignore
//     userMsgSet.get(1).set(Math.round(Math.random() * 1000), timestamp())
// }, 2000)
//
// let a = 18446744073709551615
// console.log(a)

console.log(new Cmd('aaa', Array.from([(await Msg.withText('bbb', 123)).toUint8Array()])))


