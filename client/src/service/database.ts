import { invoke } from "@tauri-apps/api"
import { Msg } from "../entity/msg";

class MsgDB {
    static saveMsg = async (msgList: Array<Msg>): Promise<void> => {
        let list = msgList.map((msg) => {
            return msg.toArrayBuffer();
        });
        await invoke<void>("save_msg", {
            params: {
                msg_list: list,
            }
        });
    }

    static getMsgList = async (userId: bigint, peerId: bigint, seqNumFrom: bigint, seqNumEnd: bigint): Promise<Array<Msg>> => {
        let list = await invoke<Array<any>>("get_msg_list", {
            params: {
                user_id: userId.toString(),
                peer_id: peerId.toString(),
                seq_num_from: seqNumFrom.toString(),
                seq_num_end: seqNumEnd.toString(),
            }
        });
        return list.map((item) => {
            let array = item as Array<number>;
            let body = new Uint8Array(array.length);
            for (let i = 0; i < array.length; i++) {
                body[i] = array[i];
            }
            return Msg.fromArrayBuffer(body.buffer);
        });
    }

    static getMsg = async (userId: bigint, peerId: bigint, seqNum: bigint): Promise<Msg | undefined> => {
        let msg = await invoke<Array<number> | undefined>("get_msg", {
            params: {
                user_id: userId.toString(),
                peer_id: peerId.toString(),
                seq_num: seqNum.toString(),
            }
        });
        if (msg) {
            let body = new Uint8Array(msg.length);
            for (let i = 0; i < msg.length; i++) {
                body[i] = msg[i];
            }
            return Msg.fromArrayBuffer(body.buffer);
        } else {
            return undefined;
        }
    }
}

class KVDB {
    static get = async <T>(key: string): Promise<T | undefined> => {
        try {
            let val = await invoke<string>("get_kv", {
                params: {
                    key: key,
                }
            });
            return JSON.parse(val) as T;
        } catch (e) {
            return undefined;
        }
    }

    static set = async <T>(key: string, value: T): Promise<T | undefined> => {
        try {
            let val = await invoke<string>("set_kv", {
                params: {
                    key: key,
                    val: JSON.stringify(value),
                }
            });
            return JSON.parse(val) as T;
        } catch (e) {
            return undefined;
        }
    }

    static del = async <T>(key: string): Promise<T | undefined> => {
        try {
            let val = await invoke<string>("del_kv", {
                params: {
                    key: key,
                }
            }) as string;
            return JSON.parse(val) as T;
        } catch (e) {
            return undefined;
        }
    }
}

export {
    MsgDB,
    KVDB,
}