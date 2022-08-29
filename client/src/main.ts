import {createApp, watch} from 'vue'
import './style.css'
import App from './App.vue'
import router from './router/index'
import store from './store/index'
import {Head, Msg} from "./api/backend/entity";
import {put_suitable, withId} from "./system/net";
import {timestamp} from "./util/base";

createApp(App).use(router).use(store).mount('#app')

watch(withId, (n, o) => {
    console.log('main', n)
})

let t = performance.now();
console.log(t + ':' + t.toFixed(6))
setTimeout(() => {
    let t = performance.now();
    console.log(t + ':' + t.toFixed(6))

}, 2000)

