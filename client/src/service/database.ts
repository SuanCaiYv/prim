import { invoke } from "@tauri-apps/api"

class MsgDB {
}

class KVDB {
    static get = async (key: string): Promise<string | undefined> => {
        try {
            let val = await invoke("get_kv", {
                params: {
                    key: key,
                }
            }) as string;
            return val;
        } catch (e) {
            return undefined;
        }
    }

    static set = async (key: string, value: string): Promise<string | undefined> => {
        try {
            let val = await invoke("set_kv", {
                params: {
                    key: key,
                    val: value,
                }
            }) as string;
            return val;
        } catch (e) {
            return undefined;
        }
    }

    static del = async (key: string): Promise<string | undefined> => {
        try {
            let val = await invoke("del_kv", {
                params: {
                    key: key,
                }
            }) as string;
            return val;
        } catch (e) {
            return undefined;
        }
    }
}

export {
    MsgDB,
    KVDB,
}