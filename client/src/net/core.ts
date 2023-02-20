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

    constructor(remoteAddress: string, token: string, mode: string, userId: bigint, nodeId: number) {
        this.remoteAddress = remoteAddress;
        this.token = token;
        this.mode = mode;
        this.userId = userId;
        this.nodeId = nodeId;
        this.queue = new BlockQueue<Msg>();
    }

    connect(): Promise<void> {
        invoke("connect", {
            params: {
                address: this.remoteAddress,
                token: this.token,
                mode: this.mode,
                user_id: this.userId,
                node_id: this.nodeId,
            }
        }).then((_) => {
            let _unListen = appWindow.listen("recv", (event) => {
                let body = event.payload as ArrayBuffer;
                let msg = Msg.fromArrayBuffer(body);
                this.queue.push(msg);
            })
        }).catch((err) => {
            console.log(err);
            return Promise.reject();
        })
        console.log("connected to server");
        return Promise.resolve();
    }

    disconnect(): Promise<void> {
        return Promise.resolve();
    }

    send(msg: Msg): Promise<void> {
        return invoke("send", {
            params: {
                raw: [...new Uint8Array(msg.toArrayBuffer())]
            }
        })
    }

    // should be invoked multi times.
    recv(): Promise<Msg> {
        return this.queue.pop();
    }
}

export { Client };