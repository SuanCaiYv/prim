import {appWindow} from '@tauri-apps/api/window'
import {Cmd, Msg} from './entity';
import {i64ToByteArray} from '../../util/base';
import {get} from "idb-keyval";
import {UnlistenFn} from "@tauri-apps/api/event";

const SERVER_ADDRESS = '121.5.137.55:8190'

let unlistenFunc: UnlistenFn;

class Client {
    public address: string

    constructor(address: string) {
        this.address = address;
    }

    public async connect() {
        await appWindow.emit("connect", this.address)
    }

    public async close() {
        await appWindow.emit("cmd", new Cmd("close", Array.from([i64ToByteArray(0)])).toObj());
    }

    public async heartbeat() {
        const accountId = await get('AccountId');
        await appWindow.emit("cmd", new Cmd("heartbeat", Array.from([i64ToByteArray(accountId)])).toObj());
    }

    public async send(cmd: Cmd) {
        await appWindow.emit("cmd", cmd.toObj())
    }

    public async send_msg(msg: Msg) {
        await this.send(new Cmd("send-msg", Array.from([msg.toUint8Array()])))
    }

    public async recv(handler: Function) {
        // 取消上次监听
        if (unlistenFunc !== undefined && unlistenFunc !== null) {
            console.log('already un listen last callback')
            unlistenFunc()
        }
        unlistenFunc = await appWindow.listen("cmd-res", event => {
            handler(Cmd.fromObj(event.payload))
        })
    }
}

export {Client, SERVER_ADDRESS}