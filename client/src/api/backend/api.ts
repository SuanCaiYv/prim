import {appWindow} from '@tauri-apps/api/window'
import {Cmd, Msg} from "./entity";
import {i64ToByteArray} from "../../util/base";
import {get} from "idb-keyval";

class Client {
    public address: string

    constructor(address: string) {
        this.address = address;
    }

    public async connect() {
        await appWindow.emit("connect", this.address)
    }

    public async close() {
        await appWindow.emit("cmd", new Cmd("close", []).toObj());
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

    public async refresh() {
        await appWindow.emit("cmd", new Cmd("refresh", Array.from([])).toObj());
    }

    public async recv(handler: Function) {
        await appWindow.listen("cmd-res", event => {
            handler(Cmd.fromObj(event.payload))
        })
    }
}

export {Client}