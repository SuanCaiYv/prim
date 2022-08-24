<script setup lang="ts">
import Layout from './Layout.vue'
import {useRouter} from "vue-router"
import {BASE_URL, httpClient} from "../../api/frontend";
import alertFunc from "../../util/alert";
import {Cmd, Msg, SyncArgs, Type} from "../../api/backend/entity";
import {Client} from "../../api/backend/api";
import {inject, provide, reactive, ref} from "vue";
import {get, set} from "idb-keyval";

const router = useRouter()
const msgBox = reactive<Array<number>>([])
const msgChannel = reactive<Map<number, Array<Msg>>>(new Map())
const chatList = reactive<Array<number>>([])
let addFriendList = inject('addFriendList') as Array<any>
let accountId = get('AccountId')

provide('chatList', chatList)
provide('msgChannel', msgChannel)

get('ChatList').then(list => {
    if (list !== undefined) {
        let l: Array<number> = list
        for (let i = 0; i < l.length; i++) {
            chatList.push(l[i])
        }
    }
})
get('MsgChannel').then(channel => {
    if (channel !== undefined) {
        let chan: Map<number, Array<Msg>> = channel;
        chan.forEach((value, key) => {
            msgChannel.set(key, value)
        })
    }
})
setInterval(() => {
    set('MsgChannel', msgChannel);
    set('ChatList', chatList);
}, 3000)

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
            console.log(resp.data)
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
    switch (msg.head.typ) {
        case Type.Text || Type.Meme || Type.File || Type.Image || Type.Audio || Type.Video:
            let list = msgChannel.get(await getReceiver(msg.head.sender, msg.head.receiver));
            if (list !== undefined) {
                put_suitable(msg, list)
            } else {
                list = new Array<Msg>()
                put_suitable(msg, list)
                msgChannel.set(await getReceiver(msg.head.sender, msg.head.receiver), list)
            }
            break;
        case Type.Box:
            const accountList: Array<number> = JSON.parse(msg.payload);
            for (let accountId in accountList) {
                msgBox.push(Number(accountId))
                chatList.push(Number(accountId))
                await netApi.send_msg(await Msg.sync(new SyncArgs(0, true, 0), Number(accountId)))
            }
            break;
        case Type.Error:
            break;
        case Type.Offline:
            break;
        case Type.FriendRelationship:
            let payload = msg.payload.split("_")
            addFriendList.push({
                remark: payload[1],
                userId: msg.head.sender,
            })
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

const netApi = new Client("127.0.0.1:8190")
netApi.connect().then(async () => {
    await netApi.send_msg(await Msg.auth());
    await netApi.heartbeat();
    await netApi.recv(handler)
    await netApi.send_msg(await Msg.box())
})
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
