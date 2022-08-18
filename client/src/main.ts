import { createApp } from 'vue'
import { emit, listen } from '@tauri-apps/api/event'
import './style.css'
import App from './App.vue'
import router from './router/index'

createApp(App).use(router).mount('#app')

emit('connect', "127.0.0.1:8190").then(r => {
    console.log(r)
})

const unlisten = await listen('recv-msg', (event) => {
    console.log(event)
})

console.log('bbb')

setInterval(() => {
    emit('send-msg', 'qwer').then(r => {
        console.log(r)
    })
}, 2000)
