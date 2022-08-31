import {createApp} from 'vue'
import './style.css'
import App from './App.vue'
import store from "./store";
import router from "./route";
import {hock} from "./function/net";

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

