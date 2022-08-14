import {contextBridge, ipcRenderer} from "electron";

contextBridge.exposeInMainWorld('electronApi', {
    connect: () => ipcRenderer.send('net', 'connect')
})