<script setup lang="ts">
import Layout from './Layout.vue'
import {useRouter} from "vue-router"
import {BASE_URL, httpClient} from "../../api/frontend";
import alertFunc from "../../util/alert";
import {Cmd, Head, Msg, SyncArgs, Type} from "../../api/backend/entity";
import {Client} from "../../api/backend/api";
import {inject, provide, reactive, Ref, ref, watch, watchEffect} from "vue";
import {clear, get, set} from "idb-keyval";
import {KV} from "../../api/frontend/interface";
import {useStore} from "vuex";

const router = useRouter()
let msgChannel = reactive<Map<number, Array<Msg>>>(new Map<number, Array<Msg>>())
let chatList= reactive<Array<number>>([])
let chatSet = inject('chatSet') as Set<number>
let sendMsgChannel = inject('sendMsgChannel') as Array<KV<string, number>>
let accountId = get('AccountId')
const store = useStore();

provide('chatList', chatList)
provide('msgChannel', msgChannel)

watchEffect(async () => {
    if (sendMsgChannel.length > 0) {
        const len = sendMsgChannel.length;
        await netApi.send_msg(await Msg.withText(sendMsgChannel[len-1].key, sendMsgChannel[len-1].value))
        sendMsgChannel.splice(len-1, 1)
    }
})

watchEffect(() => {
    let arr = Array.from(chatSet).sort((a, b) => {
        let l1 = msgChannel.get(a);
        let l2 = msgChannel.get(b);
        if (l1 === undefined || l1.length === 0) {
            return 1
        }
        if (l2 === undefined || l2.length === 0) {
            return -1
        }
        return l1[0].head.timestamp < l2[0].head.timestamp ? 1 : -1
    })
    chatList.splice(0, chatList.length, ...arr)
})

get('ChatList').then(list => {
    if (list !== undefined) {
        let l: Array<number> = list
        for (let i = 0; i < l.length; i++) {
            chatSet.add(l[i])
        }
    }
})
get('MsgChannel').then(channel => {
    if (channel !== undefined) {
        let chan = channel as Map<number, Array<Msg>>
        for (let [key, value] of chan) {
            msgChannel.set(key, value)
        }
    }
})
// setInterval(() => {
//     let msgChan = new Map<number, Array<Msg>>()
//     for (let [key, value] of msgChannel.entries()) {
//         let arr = new Array<Msg>(value.length)
//         for (let i = 0; i < arr.length; i++) {
//             arr[i] = new Msg(new Head(value[i].head.length, value[i].head.typ, value[i].head.sender, value[i].head.receiver, value[i].head.timestamp, value[i].head.seq_num, value[i].head.version), value[i].payload)
//         }
//         msgChan.set(key, arr)
//     }
//     let list = new Array<number>(chatList.length)
//     for (let i = 0; i < chatList.length; i++) {
//         list[i] = chatList[i]
//     }
//     set('MsgChannel', msgChan);
//     set('ChatList', list);
// }, 3000)

get('Authed').then(async authed => {
    if (!authed) {
        await router.push('/sign')
    } else {
        await router.push('/')
    }
})

accountId.then(accountId => {
    httpClient.get('/user/info/' + accountId, {}, true).then(resp => {
        if (!resp.ok) {
            alertFunc(resp.errMsg, function () {
                router.push('/sign')
            })
        } else {
            // @ts-ignore
            set('AccountAvatar', BASE_URL + resp.data.avatar)
        }
    })
})

const getReceiver = async (id1: number, id2: number): Promise<number> => {
    if (id1 === await accountId) {
        return id2;
    } else {
        return id1;
    }
}
const put_suitable= async (msg: Msg, array: Array<Msg>) => {
    if (array.length === 0) {
        array.push(msg)
        return
    } else {
        let index = array.length-1;
        for (index; index >= 0; index --) {
            if (array[index].head.seq_num < msg.head.seq_num) {
                array.splice(index, 0, msg)
                return
            } else if (array[index].head.seq_num === msg.head.seq_num) {
                return
            }
        }
    }
}

const connectHandler = async (cmd: Cmd) => {
    if (String(cmd.args[0]) === 'false') {} else {}
}
const msgHandler = async (cmd: Cmd) => {
    let msg = Msg.fromUint8Array(cmd.args[0])
    console.log('msg', msg)
    switch (msg.head.typ) {
        case Type.Text || Type.Meme || Type.File || Type.Image || Type.Audio || Type.Video:
            let list1 = msgChannel.get(await getReceiver(msg.head.sender, msg.head.receiver));
            console.log('before', list1)
            if (list1 !== undefined) {
                list1.push(msg)
            } else {
                list1 = reactive(new Array<Msg>())
                list1.push(msg)
                msgChannel.set(await getReceiver(msg.head.sender, msg.head.receiver), list1)
            }
            console.log('after', list1)
            break;
        case Type.Box:
            const accountList: Array<number> = JSON.parse(msg.payload);
            for (let i = 0; i < accountList.length; i ++) {
                chatSet.add(Number(accountList[i]))
                await netApi.send_msg(await Msg.sync(new SyncArgs(0, true, 0), Number(accountList[i])))
            }
            break;
        case Type.Error:
            break;
        case Type.Offline:
            break;
        case Type.FriendRelationship:
            let payload = msg.payload;
            if (payload.startsWith('ADD_')) {
                let list2 = msgChannel.get(await getReceiver(msg.head.sender, msg.head.receiver));
                if (list2 !== undefined) {
                    list2.push(msg)
                } else {
                    list2 = reactive(new Array<Msg>())
                    list2.push(msg)
                    msgChannel.set(await getReceiver(msg.head.sender, msg.head.receiver), list2)
                }
            }
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

const moreMsg = async (friendId: number) => {
    let list = msgChannel.get(friendId)
    if (list === undefined) {
        list = new Array<Msg>()
        msgChannel.set(friendId, list)
    }
    const tail = list[list.length - 1];
    let sync = await Msg.sync(new SyncArgs(tail.head.seq_num-1, true, 20), friendId)
    await netApi.send_msg(sync)
}

provide('moreMsg', moreMsg)

let netApi: Client;

if (!store.getters.connected) {
    netApi = new Client("127.0.0.1:8190")
    netApi.connect().then(async () => {
        await netApi.recv(handler)
        store.commit('updateConnected', true)
        store.commit('updateNetApi', netApi)
    })
} else {
    netApi = store.getters.netApi
}
</script>

<template>
    <div class="home">
        <Layout></Layout>
    </div>
</template>

<style scoped>
.home {
    width: 100%;
    height: 100%;
}
</style>
