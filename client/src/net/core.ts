import { UnlistenFn } from '@tauri-apps/api/event';
import { Msg } from "../entity/msg";
import { appWindow } from "@tauri-apps/api/window"
import { BlockQueue } from "../util/queue";
import { invoke } from "@tauri-apps/api";

class Client {
    remoteAddress: string;
    token: string;
    mode: string;
    userId: bigint;
    nodeId: number;
    queue: BlockQueue<Msg>;
    recvCb: (msg: Msg) => void | undefined;
    unListen: UnlistenFn;

    constructor(remoteAddress: string, token: string, mode: string, userId: bigint, nodeId: number, recvCb: (msg: Msg) => void | undefined) {
        this.remoteAddress = remoteAddress;
        this.token = token;
        this.mode = mode;
        this.userId = userId;
        this.nodeId = nodeId;
        this.queue = new BlockQueue<Msg>();
        this.recvCb = recvCb;
        this.unListen = () => { };
    }

    connect = async () => {
        try {
            await invoke("connect", {
                params: {
                    address: this.remoteAddress,
                    token: this.token,
                    mode: this.mode,
                    user_id: this.userId,
                    node_id: this.nodeId,
                }
            })
        } catch (e) {
            console.log(e);
            throw e;
        }
        if (this.recvCb !== undefined) {
            this.unListen = await appWindow.listen<Array<number>>("recv", (event) => {
                let body = new Uint8Array(event.payload.length);
                for (let i = 0; i < event.payload.length; ++ i) {
                    body[i] = event.payload[i];
                }
                let msg = Msg.fromArrayBuffer(body.buffer);
                this.recvCb(msg);
            })
        } else {
            this.unListen = await appWindow.listen<Array<number>>("recv", (event) => {
                let body = new Uint8Array(event.payload.length);
                for (let i = 0; i < event.payload.length; ++ i) {
                    body[i] = event.payload[i];
                }
                let msg = Msg.fromArrayBuffer(body.buffer);
                this.queue.push(msg);
            })
        }
        console.log("connected to server");
        return;
    }

    disconnect = async () => {
        this.unListen();
        try {
            await invoke("disconnect", {});
        } catch (e) {
            console.log(e);
            return;
        }
        console.log("disconnected from server");
    }

    send = async (msg: Msg) => {
        try {
            invoke("send", {
                params: {
                    raw: [...new Uint8Array(msg.toArrayBuffer())]
                }
            })
        } catch (e) {
            console.log(e);
            throw e;
        }
        return;
    }

    // should be invoked multi times.
    recv = async (): Promise<Msg> => {
        if (this.recvCb !== undefined) {
            return Promise.reject("recv callback is set");
        }
        return this.queue.pop();
    }
}

export { Client };