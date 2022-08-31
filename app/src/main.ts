import {createApp} from 'vue'
import './style.css'
import App from './App.vue'
import store from "./store";
import router from "./route";
import {hock, startNet} from "./function/net";
import {set} from "idb-keyval";
import {Constant} from "./system/constant";

createApp(App).use(store).use(router).mount('#app')

hock.value = false

set(Constant.AccountId, 1)
set(Constant.Token, '0x0987654321')
startNet()
