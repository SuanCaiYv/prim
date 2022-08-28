import {Client} from '../api/backend/net';
import store from "../store";
import {Cmd, Msg, SyncArgs, Type} from "../api/backend/entity";
import {get} from "idb-keyval";
import {
    msgChannelMap,
    msgChannelMapNewest,
    msgChannelMapNext,
    msgChannelMapSynced, sendMsgChannel,
    userMsgList,
    userMsgSet
} from "./types";
import {ref, watch} from "vue";
import {Constant} from "../system/constant";
import alertFunc from "../components/alert/alert";
import {byteArrayToI64, whichIsNotMe, whoWeAre} from "../util/base";

let netApi: Client

const startNet = async () => {
    console.log(Boolean(store.getters.connected))
    if (Boolean(store.getters.connected)) {
        console.log('close net')
        // 断掉之前的连接，清理状态
        if (netApi !== undefined && netApi !== null) {
            await netApi.close()
            console.log('already closed previous net')
        }
    }
    netApi = new Client("127.0.0.1:8190")
    netApi.connect().then(async (_) => {
        console.log('connected')
        netApi.send_msg(await Msg.auth()).then(async (_) => {
            await netApi.recv(handler)
            // await netApi.heartbeat()
            await netApi.send_msg(await Msg.box())
            store.commit('updateConnected', true)
            await syncUserMsgList()
        })
    })
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
    userMsgList.splice(0, userMsgList.length)
    for (let [accountId, timestamp] of entries) {
        userMsgList.push({key: accountId, value: timestamp})
        if (!Boolean(msgChannelMapSynced.get(await msgChannelMapKey(accountId)))) {
            msgChannelMapSynced.set(await msgChannelMapKey(accountId), true)
            await syncMsg(accountId)
        }
    }
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
        await netApi.send_msg(msg)
    }
})

const connectHandler = async (cmd: Cmd) => {
    if (String(cmd.args[0]) === 'false') {
        alertFunc('连接失败')
    }
}
const msgHandler = async (cmd: Cmd) => {
    let msg = Msg.fromUint8Array(cmd.args[0])
    switch (msg.head.typ) {
        case Type.Text || Type.Meme || Type.File || Type.Image || Type.Audio || Type.Video:
            console.log('msg', msg)
            let newSet = userMsgSet.get((await get(Constant.AccountId)) as number)
            if (newSet === undefined) {
                newSet = new Map<number, number>()
                userMsgSet.set((await get(Constant.AccountId)) as number, newSet)
            }
            // @ts-ignore
            userMsgSet.get((await get(Constant.AccountId)) as number).set(msg.head.sender, msg.head.timestamp)

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
        case Type.Box:
            console.log('box', msg)
            const arr = JSON.parse(msg.payload) as Array<Array<number>>
            let set = userMsgSet.get((await get(Constant.AccountId)) as number)
            if (set === undefined) {
                set = new Map<number, number>()
                userMsgSet.set((await get(Constant.AccountId)) as number, set)
            }
            for (let i = 0; i < arr.length; i++) {
                let [accountId, timestamp] = arr[i]
                // @ts-ignore
                userMsgSet.get((await get(Constant.AccountId)) as number).set(accountId, timestamp)
            }
            break;
        case Type.Sync:
            console.log('sync', msg)
            const len = byteArrayToI64(new TextEncoder().encode(msg.payload))
            if (len === 0) {
                msgChannelMapNext.set(await msgChannelMapKey(msg.head.sender, msg.head.receiver), 0)
            }
            break;
        case Type.FriendRelationship:
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
    console.log(text)
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
            if (arr[i].head.seq_num === 0) {
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

export {hock, msgChannelMapKey, startNet}