<script setup lang="ts">
import Layout from './Layout.vue'
import {useRouter} from "vue-router"
import {httpClient} from "../../api/frontend";
import alertFunc from "../../util/alert";
import {Cmd, Msg, SyncArgs, Type} from "../../api/backend/entity";
import {Client} from "../../api/backend/api";
import {reactive} from "vue";
import {List} from "../../util/list";
import {get, set} from "idb-keyval";

const router = useRouter()
const msgBox = reactive<Array<number>>([])
const msgChannel = reactive<Map<number, List<Msg>>>(new Map())
let accountId = get('AccountId')

get('MsgChannel').then(channel => {
    if (channel !== undefined) {
        let chan: Map<number, List<Msg>> = channel;
        chan.forEach((value, key) => {
            msgChannel.set(key, value)
        })
    }
})
setInterval(() => {
    set('MsgChannel', msgChannel)
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

const connectHandler = async (cmd: Cmd) => {
    if (String(cmd.args[0]) === 'false') {} else {}
}
const msgHandler = async (cmd: Cmd) => {
    let msg = Msg.fromUint8Array(cmd.args[0])
    switch (msg.head.typ) {
        case Type.Text || Type.Meme || Type.File || Type.Image || Type.Audio || Type.Video:
            let list = msgChannel.get(await getReceiver(msg.head.sender, msg.head.receiver));
            if (list !== undefined) {
                list.push_front_suit(msg, msg.head.seq_num)
            } else {
                list = new List<Msg>()
                list.push_front_suit(msg, msg.head.seq_num)
                msgChannel.set(await getReceiver(msg.head.sender, msg.head.receiver), list)
            }
            break;
        case Type.Box:
            const accountList: Array<number> = JSON.parse(msg.payload);
            for (let accountId in accountList) {
                msgBox.push(Number(accountId))
                await netApi.send_msg(await Msg.sync(new SyncArgs(0, true, 0), Number(accountId)))
            }
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

const moreMsg = async (friendId: number) => {
    let list = msgChannel.get(friendId)
    if (list === undefined) {
        return
    }
    const head = list.head;
    const msg: Msg = head?.value;
    let sync = await Msg.sync(new SyncArgs(msg.head.seq_num-1, true, 20), friendId)
    await netApi.send_msg(sync)
}

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
