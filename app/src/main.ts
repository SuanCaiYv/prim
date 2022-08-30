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
import {hock, startNet} from "./function/net";
import {set} from "idb-keyval";
import {Cmd, Msg} from "./api/backend/entity";

createApp(App).use(store).use(router).mount('#app')

hock.value = false

// userMsgSet.set(1, new Map<number, number>())
// setTimeout(() => {
//     // @ts-ignore
//     userMsgSet.get(1).set(Math.round(Math.random() * 1000), Math.round(Math.random() * 1000))
//     setTimeout(() => {
//         // @ts-ignore
//         userMsgSet.get(1).set(Math.round(Math.random() * 1000), Math.round(Math.random() * 1000))
//         setTimeout(() => {
//             // @ts-ignore
//             userMsgSet.get(1).set(Math.round(Math.random() * 1000), Math.round(Math.random() * 1000))
//             setTimeout(() => {
//                 // @ts-ignore
//                 userMsgSet.get(1).set(Math.round(Math.random() * 1000), Math.round(Math.random() * 1000))
//                 setTimeout(() => {
//                     // @ts-ignore
//                     userMsgSet.get(1).set(Math.round(Math.random() * 1000), Math.round(Math.random() * 1000))
//                 }, 2000)
//             }, 2000)
//         }, 2000)
//     }, 2000)
// }, 2000)

// console.log(JSON.stringify(new Cmd('send-msg', Array.from([(await Msg.withText('bbb', 123)).toUint8Array()])).toObj()))

let str = "ADD_aaa"
console.log(str.split('_')[1])
