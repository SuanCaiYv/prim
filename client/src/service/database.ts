import { invoke } from "@tauri-apps/api"
import { Msg } from "../entity/msg";

class MsgDB {
    static saveMsgList = async (msgList: Array<Msg>): Promise<void> => {
        let list = new Array<Array<number>>();
        for (let msg of msgList) {
            list.push([...new Uint8Array(msg.toArrayBuffer())]);
        }
        try {
            await invoke<void>("save_msg_list", {
                params: {
                    msg_list: list,
                }
            });
        } catch (e) {
            console.log(e);
        }
    }

    static saveMsg = async (msg: Msg): Promise<void> => {
        try {
            await invoke<void>("save_msg", {
                params: {
                    msg: [...new Uint8Array(msg.toArrayBuffer())],
                }
            });
        } catch (e) {
            console.log(e);
        }
    }

    static getMsgList = async (userId: bigint, peerId: bigint, seqNumFrom: bigint, seqNumTo: bigint): Promise<Array<Msg>> => {
        let list = await invoke<Array<any>>("get_msg_list", {
            params: {
                user_id: userId.toString(),
                peer_id: peerId.toString(),
                seq_num_from: seqNumFrom.toString(),
                seq_num_to: seqNumTo.toString(),
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

    static latestSeqNum = async (userId: bigint, peerId: bigint): Promise<bigint> => {
        try {
            let seqNum = await invoke<string>("latest_seq_num", {
                params: {
                    user_id: Number(userId),
                    peer_id: Number(peerId),
                }
            }) as string;
            return BigInt(seqNum);
        } catch (e) {
            console.log(e);
            return 0n;
        };
    }
}

class KVDB {
    static get = async (key: string): Promise<any | undefined> => {
        try {
            let val = await invoke<any>("get_kv", {
                params: {
                    key: key,
                }
            });
            return val;
        } catch (e) {
            console.log(e);
            return undefined;
        }
    }

    static set = async (key: string, value: any): Promise<any | undefined> => {
        try {
            let val = await invoke<any>("set_kv", {
                params: {
                    key: key,
                    val: value,
                }
            });
            return val;
        } catch (e) {
            console.log(e);
            return undefined;
        }
    }

    static del = async (key: string): Promise<any | undefined> => {
        try {
            let val = await invoke<string>("del_kv", {
                params: {
                    key: key,
                }
            });
            return val
        } catch (e) {
            console.log(e);
            return undefined;
        }
    }
}

export {
    MsgDB,
    KVDB,
}