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
    unListen: UnlistenFn;

    constructor(remoteAddress: string, token: string, mode: string, userId: bigint, nodeId: number) {
        this.remoteAddress = remoteAddress;
        this.token = token;
        this.mode = mode;
        this.userId = userId;
        this.nodeId = nodeId;
        this.queue = new BlockQueue<Msg>();
        this.unListen = () => {};
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
        this.unListen = await appWindow.listen("recv", (event) => {
            let body = event.payload as ArrayBuffer;
            let msg = Msg.fromArrayBuffer(body);
            this.queue.push(msg);
        })
        console.log("connected to server");
        return;
    }

    disconnect = async () => {
        this.unListen();
        return;
    }

    send = async (msg: Msg) => {
        try { invoke("send", {
            params: {
                raw: [...new Uint8Array(msg.toArrayBuffer())]
            }
        })} catch (e) {
            console.log(e);
            throw e;
        }
        return;
    }

    // should be invoked multi times.
    recv = async (): Promise<Msg> => {
        return this.queue.pop();
    }
}

export { Client };