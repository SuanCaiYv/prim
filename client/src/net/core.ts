import { Msg } from "../entity/msg";
import {appWindow} from "@tauri-apps/api/window"

class Client {
    remoteAddress: string;
    token: string;
    mode: string;
    userId: bigint;
    nodeId: number;

    constructor(remoteAddress: string, token: string, mode: string, userId: bigint, nodeId: number) {
        this.remoteAddress = remoteAddress;
        this.token = token;
        this.mode = mode;
        this.userId = userId;
        this.nodeId = nodeId;
    }

    connect(): Promise<void> {}

    disconnect(): Promise<void> {}

    send(msg: Msg): Promise<void> {}

    recv(): Promise<Msg> {
        appWindow.listen("recv", (event) => {})
    }
}

export { Client };