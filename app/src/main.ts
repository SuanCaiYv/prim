import {createApp} from 'vue'
import './style.css'
import App from './App.vue'
import store from "./store";
import router from "./route";
import {hock} from "./function/net";

createApp(App).use(store).use(router).mount('#app')

hock.value = false
