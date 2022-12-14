import {Client} from '../api/backend/net';
import store from "../store";
import {Cmd, Msg, SyncArgs, Type} from "../api/backend/entity";
import {get} from "idb-keyval";
import {
    msgChannelMap,
    msgChannelMapNewest,
    msgChannelMapNext,
    msgChannelMapSynced,
    sendMsgChannel,
    userMsgList,
    userMsgSet
} from "./types";
import {ref, watch} from "vue";
import {Constant} from "../system/constant";
import alertFunc from "../components/alert/alert";
import {byteArrayToI64, whoWeAre} from "../util/base";
import {KV} from "../api/frontend/entity";
import {SERVER_ADDRESS} from "../api/backend/net";

let netApi: Client

const tryClosePreviousNet = async () => {
    if (Boolean(store.getters.connected)) {
        console.log('close net')
        // 断掉之前的连接，清理状态
        if (netApi !== undefined && netApi !== null) {
            await netApi.close()
            console.log('already closed previous net')
        }
    }
}

const startNet = async () => {
    netApi = new Client(SERVER_ADDRESS)
    await netApi.connect()
    await netApi.recv(handler)
}

const syncUserMsgList = async () => {
    let entries = userMsgSet.get((await get(Constant.AccountId)) as number)
    if (entries === undefined) {
        return
    }
    for (let [accountId, _] of entries) {
        await syncMsg(accountId)
    }
}

// 每次调用都会拉取50条消息
const syncMsg = async (withAccountId: number) => {
    if (await nextSeqNum(withAccountId) <= await newestSeqNum(withAccountId)) {
        return
    }
    let args = new SyncArgs(await nextSeqNum(withAccountId), true, 50)
    let msg = await Msg.sync(args, withAccountId)
    await netApi.send_msg(msg)
    msgChannelMapSynced.set(await msgChannelMapKey(withAccountId), true)
}

// 获取持久化数据
get(Constant.MsgChannelMap).then(msgChannelMap0 => {
    if (msgChannelMap0 !== undefined) {
        let m = msgChannelMap0 as Map<string, Array<Msg>>
        let entries = m.entries()
        for (let [key, value] of entries) {
            msgChannelMap.set(key, value)
        }
    }
})

get(Constant.UserMsgSet).then(userMsgSet0 => {
    if (userMsgSet0 !== undefined) {
        let map = userMsgSet0 as Map<number, Map<number, number>>
        let entries = map.entries();
        for (let [key, value] of entries) {
            let m = value.entries()
            let mmap = new Map<number, number>()
            for (let [k, v] of m) {
                mmap.set(k, v)
            }
            userMsgSet.set(key, mmap)
        }
    }
})

// 获取上次同步的最新序列号，且无论获取多少次都是同一个值
const newestSeqNum = async (withAccountId: number): Promise<number> => {
    let key = await msgChannelMapKey(withAccountId)
    let ans = msgChannelMapNewest.get(key)
    if (ans !== undefined) {
        return ans
    }
    let arr = msgChannelMap.get(key)
    if (arr === undefined || arr.length === 0) {
        msgChannelMap.set(key, new Array<Msg>())
        ans = 0
    } else {
        ans = arr[arr.length - 1].head.seq_num
    }
    msgChannelMapNewest.set(key, ans)
    return ans
}

const nextSeqNum = async (withAccountId: number): Promise<number> => {
    let key = await msgChannelMapKey(withAccountId)
    let ans = msgChannelMapNext.get(key)
    if (ans === undefined) {
        return 4294967295
    } else {
        return ans
    }
}

watch(userMsgSet, async (mapSet, _) => {
    let set = mapSet.get((await get(Constant.AccountId)) as number)
    if (set === undefined) {
        return
    }
    let entries = set.entries()
    let arr = new Array<KV<number, number>>()
    for (let [accountId, timestamp] of entries) {
        arr.push(new KV<number, number>(accountId, timestamp))
        const key = await msgChannelMapKey(accountId)
        if (!Boolean(msgChannelMapSynced.get(key))) {
            msgChannelMapSynced.set(key, true)
            await syncMsg(accountId)
        }
    }
    userMsgList.value = arr
})

watch(userMsgList, (msgList, _) => {
    msgList.sort((a, b) => {
        return a.value - b.value
    })
})

watch(sendMsgChannel, async (channel, _) => {
    if (channel.length > 0) {
        const msg = channel[channel.length-1]
        channel.splice(channel.length-1, 1)
        let key = await msgChannelMapKey(msg.head.sender, msg.head.receiver)
        pushSuitable(msg, key)
        await netApi.send_msg(msg)
    }
})

const getNetApi = (): Client => {
    return netApi
}

const connectHandler = async (cmd: Cmd) => {
    if (String(cmd.args[0]) === 'false') {
        alertFunc('连接失败')
    } else {
        console.log('connected')
        // 禁用闭包缓存
        const api = getNetApi();
        await api.send_msg(await Msg.auth())
        // todo 开启心跳
        // await api.heartbeat()
        await api.send_msg(await Msg.box())
        store.commit('updateConnected', true)
        await syncUserMsgList()
    }
}
const msgHandler = async (cmd: Cmd) => {
    let msg = Msg.fromUint8Array(cmd.args[0])
    switch (msg.head.typ) {
        case Type.Text || Type.Meme || Type.File || Type.Image || Type.Audio || Type.Video:
            let setKey1 = (await get(Constant.AccountId)) as number
            let set1 = userMsgSet.get(setKey1)
            if (set1 === undefined) {
                set1 = new Map<number, number>()
                userMsgSet.set(setKey1, set1)
            }
            if (msg.head.sender !== setKey1) {
                // @ts-ignore
                userMsgSet.get(setKey1).set(msg.head.sender, msg.head.timestamp)
            }

            let key1 = await msgChannelMapKey(msg.head.sender, msg.head.receiver);
            let num1 = msgChannelMapNext.get(key1)
            if (num1 === undefined) {
                msgChannelMapNext.set(key1, msg.head.seq_num)
            } else {
                if (num1 < msg.head.seq_num) {
                    msgChannelMapNext.set(key1, msg.head.seq_num)
                }
            }
            let msgArr1 = msgChannelMap.get(key1)
            if (msgArr1 === undefined) {
                msgArr1 = new Array<Msg>()
                msgChannelMap.set(key1, msgArr1)
            }
            pushSuitable(msg, key1)
            break;
        case Type.Box:
            const arr = JSON.parse(msg.payload) as Array<Array<number>>
            let setKey = await get(Constant.AccountId) as number
            let set = userMsgSet.get(setKey)
            if (set === undefined) {
                set = new Map<number, number>()
                userMsgSet.set(setKey, set)
            }
            for (let i = 0; i < arr.length; i++) {
                let [accountId, timestamp] = arr[i]
                // @ts-ignore
                userMsgSet.get(setKey).set(accountId, timestamp)
            }
            break;
        case Type.Sync:
            const len = byteArrayToI64(new TextEncoder().encode(msg.payload))
            if (len === 0) {
                msgChannelMapNext.set(await msgChannelMapKey(msg.head.sender, msg.head.receiver), 0)
            }
            break;
        case Type.FriendRelationship:
            let sKey = (await get(Constant.AccountId)) as number
            let newSet = userMsgSet.get(sKey)
            if (newSet === undefined) {
                newSet = new Map<number, number>()
                userMsgSet.set(sKey, newSet)
            }
            if (msg.head.sender !== sKey) {
                // @ts-ignore
                userMsgSet.get(sKey).set(msg.head.sender, msg.head.timestamp)
            }

            let key = await msgChannelMapKey(msg.head.sender, msg.head.receiver);
            let num = msgChannelMapNext.get(key)
            if (num === undefined) {
                msgChannelMapNext.set(key, msg.head.seq_num)
            } else {
                if (num < msg.head.seq_num) {
                    msgChannelMapNext.set(key, msg.head.seq_num)
                }
            }
            let msgArr = msgChannelMap.get(key)
            if (msgArr === undefined) {
                msgArr = new Array<Msg>()
                msgChannelMap.set(key, msgArr)
            }
            pushSuitable(msg, key)
            break;
        case Type.Error:
            break;
        case Type.Offline:
            break;
        default:
            break;
    }
}
const textHandler = async (cmd: Cmd) => {
    let text = String(cmd.args[0])
}
const handler = async (cmd: Cmd) => {
    if (cmd.name === 'connect-result') {
        await connectHandler(cmd)
    } else if (cmd.name === 'recv-msg') {
        await msgHandler(cmd)
    } else if (cmd.name === 'text-str') {
        await textHandler(cmd)
    }
}

const msgChannelMapKey = async (...id: number[]): Promise<string> => {
    if (id.length === 1) {
        const accountId = await get(Constant.AccountId)
        return whoWeAre(accountId, id[0]) + '-msg_list_key'
    } else {
        return whoWeAre(id[0], id[1]) + '-msg_list_key'
    }
}

const pushSuitable = (msg: Msg, key: string) => {
    let arr = msgChannelMap.get(key);
    if (arr === undefined) {
        return;
    }
    if (arr.length === 0) {
        // @ts-ignore
        msgChannelMap.get(key).push(msg)
    } else {
        for (let i = arr.length-1; i >= 0; i --) {
            if (arr[i].head.seq_num === 0 || msg.head.seq_num === 0) {
                if (arr[i].head.timestamp < msg.head.timestamp) {
                    // @ts-ignore
                    msgChannelMap.get(key).splice(i+1, 0, msg)
                    return
                } else if (arr[i].head.timestamp === msg.head.timestamp) {
                    return
                }
            } else {
                if (arr[i].head.seq_num < msg.head.seq_num) {
                    // @ts-ignore
                    msgChannelMap.get(key).splice(i+1, 0, msg)
                    return
                } else if (arr[i].head.seq_num === msg.head.seq_num) {
                    return
                }
            }
        }
    }
}

const hock = ref<boolean>(true)

export {hock, msgChannelMapKey, startNet, tryClosePreviousNet}