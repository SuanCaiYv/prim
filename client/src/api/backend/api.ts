import {appWindow} from '@tauri-apps/api/window'
import {Msg} from "./entity";

class Client {
    public address: string

    constructor(address: string) {
        this.address = address;
    }

    public connect(): boolean {
        console.log(appWindow);
        appWindow.emit("connect", "127.0.0.1:8190")
        const unlisten = appWindow.listen("connect-result", event => {
            console.log(event)
        })

        console.log("bbb")
        // console.log(appWindow)
        // let res = false;
        // const a = appWindow.emit("connect", this.address).then(() => {
        //     console.log("connect")
        //     const b = appWindow.listen("connect-result", (result) => {
        //         console.log(result)
        //         res = Boolean(result.payload);
        //     })
        // });
        // return res;
        return false;
    }

    public async close() {
        await appWindow.emit("close").then(() => {})
    }

    public async send(msg: Msg) {
        await appWindow.emit("send-msg", Array.from(msg.toUint8Array())).then(() => {
            console.log("sent")
        })
    }

    public async recv(callback: Function) {
        await appWindow.listen("recv-msg", event => {
            callback(event.payload)
        });
    }
}

export {Client}