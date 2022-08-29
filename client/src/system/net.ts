import {reactive} from "_vue@3.2.37@vue";
import {Cmd, Head, Msg, SyncArgs, Type} from "../api/backend/entity";
import store from "../store/index";
import {Client} from "../api/backend/api";
import {byteArrayToI64, whoWhereAre} from "../util/base";
import {get, set} from "idb-keyval";
import {ref} from "vue";

// 每个用户一个set
let chatSet = reactive(new Map<number, Map<number, number>>);
// 有序版本的set
let chatList = reactive(new Array<Map<number, number>>());
// 消息列表
let msgListMap = reactive(new Map<string, Array<Msg>>());
// 发送队列
let sendMsgChannel = reactive(new Array<Msg>());
// 是否发生了消息重复
let syncMsgRepeat = reactive(new Map<string, boolean>);
// 当前获取到的最老的消息
let syncMsgOldest = reactive(new Map<string, number>);
// 没有更旧的消息可以同步
let syncMsgDone = reactive(new Map<string, boolean>);
let withId = ref<number>(0)
let netApi = store.getters.netApi;

const put_suitable = async (msg: Msg, array: Array<Msg>): Promise<boolean> => {
    if (array.length === 0) {
        array.push(msg)
        return false
    } else {
        let index = array.length - 1;
        for (index; index >= 0; index--) {
            if (array[index].head.seq_num !== 0) {
                if (array[index].head.seq_num < msg.head.seq_num) {
                    array.splice(index + 1, 0, msg)
                    return false
                } else if (array[index].head.seq_num === msg.head.seq_num) {
                    return true
                }
            } else {
                if (array[index].head.timestamp < msg.head.timestamp) {
                    console.log('index', index)
                    array.splice(index + 1, 0, msg)
                    return false
                } else if (array[index].head.timestamp === msg.head.timestamp) {
                    return true
                }
            }
        }
    }
    return false
}

// setInterval(async () => {
//     console.log(msgListMap)
//     let msgMap = new Map<string, Array<Msg>>;
//     for (let [key, value] of msgListMap) {
//         let arr = new Array<Msg>(value.length)
//         for (let i = 0; i < value.length; i++) {
//             arr[i] = new Msg(new Head(value[i].head.length, value[i].head.typ, value[i].head.sender, value[i].head.receiver, value[i].head.timestamp, value[i].head.seq_num, value[i].head.version), value[i].payload)
//         }
//         msgMap.set(key, value)
//     }
//     let msgSet = new Map<number, Map<number, number>>()
//     for (let [key, value] of chatSet.entries()) {
//         let map = new Map<number, number>;
//         for (let [k, v] of value.entries()) {
//             map.set(k, v)
//         }
//         msgSet.set(key, map)
//     }
//     await set('MsgListMap', msgMap);
//     await set('ChatSet', msgSet);
// }, 3000)

get('ChatSet').then(async list => {
    if (list !== undefined) {
        let map: Map<number, Map<number, number>> = list
        let entries = map.entries();
        for (let [key, value] of entries) {
            chatSet.set(key, value)
        }
    }
})
get('MsgListMap').then(channel => {
    if (channel !== undefined) {
        let chan = channel as Map<string, Array<Msg>>
        let entries = chan.entries();
        for (let [key, value] of entries) {
            msgListMap.set(key, value)
        }
    }
})

const msgListMapKey = async (...id: number[]): Promise<string> => {
    if (id.length === 1) {
        const accountId = await get('AccountId')
        return whoWhereAre(accountId, id[0]) + '-msg_list_key'
    } else {
        return whoWhereAre(id[0], id[1]) + '-msg_list_key'
    }
}

const connectHandler = async (cmd: Cmd) => {
    if (String(cmd.args[0]) === 'false') {
    } else {
    }
}
const msgHandler = async (cmd: Cmd) => {
    let msg = Msg.fromUint8Array(cmd.args[0])
    switch (msg.head.typ) {
        case Type.Text || Type.Meme || Type.File || Type.Image || Type.Audio || Type.Video:
            console.log('msg', msg)
            let list1 = msgListMap.get(await msgListMapKey(msg.head.sender, msg.head.receiver));
            // console.log('before', list1)
            if (list1 !== undefined) {
                console.log('msg 1')
                if (await put_suitable(msg, list1)) {
                    syncMsgRepeat.set(await msgListMapKey(msg.head.sender, msg.head.receiver), true)
                }
            } else {
                console.log('msg 2')
                list1 = new Array<Msg>()
                list1.push(msg)
                msgListMap.set(await msgListMapKey(msg.head.sender, msg.head.receiver), list1)
            }
            if (syncMsgOldest.get(await msgListMapKey(msg.head.sender, msg.head.receiver)) === undefined) {
                syncMsgOldest.set(await msgListMapKey(msg.head.sender, msg.head.receiver), msg.head.seq_num);
            } else {
                if (msg.head.seq_num < Number(syncMsgOldest.get(await msgListMapKey(msg.head.sender, msg.head.receiver)))) {
                    syncMsgOldest.set(await msgListMapKey(msg.head.sender, msg.head.receiver), msg.head.seq_num);
                }
            }
            // console.log('after', list1)
            break;
        case Type.Box:
            console.log('box', msg)
            let map = chatSet.get(<number>await get('AccountId'))
            if (map === undefined) {
                map = new Map<number, number>()
                chatSet.set(<number>await get('AccountId'), map)
            }
            const accountList: Array<Array<any>> = JSON.parse(msg.payload);
            for (let i = 0; i < accountList.length; i++) {
                map.set(Number(accountList[i][0]), Number(accountList[i][1]))
                console.log('sync')
                await netApi.send_msg(await Msg.sync(new SyncArgs(0, true, 0), Number(accountList[i][0])))
            }
            break;
        case Type.Error:
            break;
        case Type.Offline:
            break;
        case Type.FriendRelationship:
            console.log('friend', msg)
            let payload = msg.payload;
            if (payload.startsWith('ADD_')) {
                let list2 = msgListMap.get(await msgListMapKey(msg.head.sender, msg.head.receiver));
                if (list2 !== undefined) {
                    console.log('friend 1')
                    list2.push(msg)
                } else {
                    console.log('friend 2')
                    list2 = new Array<Msg>()
                    list2.push(msg)
                    msgListMap.set(await msgListMapKey(msg.head.sender, msg.head.receiver), list2)
                }
            }
            break;
        case Type.Sync:
            // console.log('sync', msg)
            const listLen = byteArrayToI64(new TextEncoder().encode(msg.payload))
            if (listLen === 0) {
                syncMsgDone.set(await msgListMapKey(msg.head.sender, msg.head.receiver), true);
            }
            break;
        default:
            break;
    }
    console.log('msg channel in net', msgListMap)
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

const moreMsg = async (withId: number) => {
    let list = msgListMap.get(await msgListMapKey(withId))
    if (list === undefined) {
        list = new Array<Msg>()
        msgListMap.set(await msgListMapKey(withId), list)
    }
    const tail = list[list.length - 1];
    let sync = await Msg.sync(new SyncArgs(tail.head.seq_num - 1, true, 20), withId)
    await netApi.send_msg(sync)
}

const startNetApi = async () => {
    console.log('connected', Boolean(store.getters.connected))
    if (Boolean(store.getters.connected)) {
        console.log('netApi', netApi)
        // 断掉之前的连接，清理状态
        if (netApi !== undefined && netApi !== null) {
            await netApi.close()
        }
    }
    netApi = new Client("127.0.0.1:8190")
    await netApi.connect()
    await netApi.send_msg(await Msg.auth())
    await netApi.recv(handler)
    await netApi.heartbeat()
    await netApi.send_msg(await Msg.box())
    store.commit('updateConnected', true)
    store.commit('updateNetApi', netApi)
    // 一些初始化操作
    let m = chatSet.get(<number>await get('AccountId'))
    if (m === undefined) {
        m = new Map<number, number>();
        chatSet.set(<number>await get('AccountId'), m)
    }
}

export {
    startNetApi, sendMsgChannel, msgListMap, chatList, chatSet, netApi, withId, moreMsg, msgListMapKey, put_suitable,
    syncMsgOldest, syncMsgRepeat, syncMsgDone
}