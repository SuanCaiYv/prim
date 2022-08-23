import {byteArrayToI16, byteArrayToI64, i16ToByteArray, i64ToByteArray, timestamp} from "../../util/base";
import storage from "../../util/storage";
import {get} from "idb-keyval";

const HEAD_LEN: number = 37;

enum Type {
    NA,
    // 消息部分
    Text,
    Meme,
    File,
    Image,
    Video,
    Audio,
    // 逻辑部分
    Ack,
    Box,
    Auth,
    Sync,
    Error,
    Offline,
    Heartbeat,
}

class Head {
    public length: number
    public typ: Type
    public sender: number
    public receiver: number
    public timestamp: number
    public seq_num: number
    public version: number

    constructor(length: number, typ: Type, sender: number, receiver: number, timestamp: number, seq_num: number, version: number) {
        this.length = length;
        this.typ = typ;
        this.sender = sender;
        this.receiver = receiver;
        this.timestamp = timestamp;
        this.seq_num = seq_num;
        this.version = version;
    }
}

class Msg {
    public head: Head
    public payload: string

    constructor(head: Head, payload: string) {
        this.head = head;
        this.payload = payload;
    }

    public toUint8Array(): Uint8Array {
        let encoder = new TextEncoder();
        let payload = encoder.encode(this.payload);
        let array = new Uint8Array(HEAD_LEN + payload.length);
        let index = 0;
        i16ToByteArray(this.head.length).forEach((v, i) => {
            array[index] = v;
            index ++;
        })
        array[index] = this.head.typ;
        index ++;
        i64ToByteArray(this.head.sender).forEach((v, i) => {
            array[index] = v;
            index ++;
        })
        i64ToByteArray(this.head.receiver).forEach((v, i) => {
            array[index] = v;
            index ++;
        })
        i64ToByteArray(this.head.timestamp).forEach((v, i) => {
            array[index] = v;
            index ++;
        })
        i64ToByteArray(this.head.seq_num).forEach((v, i) => {
            array[index] = v;
            index ++;
        })
        i16ToByteArray(this.head.version).forEach((v, i) => {
            array[index] = v;
            index ++;
        })
        payload.forEach((v, i) => {
            array[index] = v;
            index ++;
        })
        return array;
    }

    public static fromUint8Array(array: Uint8Array): Msg {
        let length = byteArrayToI16(array.slice(0, 2));
        let typ = array[2];
        let sender = byteArrayToI64(array.slice(3, 11))
        let receiver = byteArrayToI64(array.slice(11, 19))
        let timestamp = byteArrayToI64(array.slice(19, 27))
        let seq_num = byteArrayToI64(array.slice(27, 35))
        let version = byteArrayToI16(array.slice(35, 37))
        let decoder = new TextDecoder()
        let payload = decoder.decode(new Uint8Array(new Uint8Array(array.slice(37, array.length))));
        return new Msg(new Head(length, typ, sender, receiver, timestamp, seq_num, version), payload);
    }

    public static async auth(): Promise<Msg> {
        const sender = await get('AccountId')
        const token = await get('Token')
        const head = new Head(12, Type.Auth, sender, 0, timestamp(), 0, 0);
        return new Msg(head, token);
    }

    public static async sync(syncArgs: SyncArgs, withId: number): Promise<Msg> {
        const sender = await get('AccountId')
        const bytes = JSON.stringify(syncArgs)
        const head = new Head(bytes.length, Type.Sync, sender, withId, timestamp(), 0, 0);
        return new Msg(head, bytes);
    }

    public static async box(): Promise<Msg> {
        const sender = await get('account_id')
        const head = new Head(0, Type.Box, sender, 0, timestamp(), 0, 0);
        return new Msg(head, '');
    }
}

class Cmd {
    name: string
    args: Array<Uint8Array>

    constructor(name: string, args: Array<Uint8Array>) {
        this.name = name;
        this.args = args;
    }

    public toObj(): any {
        let arr = new Array(this.args.length);
        this.args.forEach((v, i) => {
            arr[i] = Array.from(v);
        });
        return {
            name: this.name,
            args: arr,
        }
    }

    public static fromObj(obj: any): Cmd {
        let arr = new Array(obj.args.length);
        // @ts-ignore
        obj.args.forEach((v, i) => {
            arr[i] = new Uint8Array(v);
        })
        return new Cmd(obj.name, arr);
    }
}

class SyncArgs {
    s: number
    b: boolean
    l: number

    constructor(seqNum: number, isBacking: boolean, length: number) {
        this.s = seqNum;
        this.b = isBacking;
        this.l = length;
    }
}

export {Type, Head, Msg, Cmd, SyncArgs}