<script setup lang="ts">
import {reactive} from "vue";
import {timestamp} from "../util/base";

let list = reactive(new Array<number>())

// watch(userMsgSet, async (mapSet, _) => {
//     console.log(mapSet)
//     let set = mapSet.get((await get(Constant.AccountId)) as number)
//     if (set === undefined) {
//         return
//     }
//     let entries = set.entries()
//     userMsgList.splice(0, userMsgList.length)
//     for (let [accountId, timestamp] of entries) {
//         userMsgList.push({key: accountId, value: timestamp})
//         if (!Boolean(msgChannelMapSynced.get(await msgChannelMapKey(accountId)))) {
//             msgChannelMapSynced.set(await msgChannelMapKey(accountId), true)
//             await syncMsg(accountId)
//         }
//     }
// })

setInterval(() => {
    list.push(1671620389584 - timestamp())
}, 1000)
</script>

<template>
    <div class="outer">
        <div class="a">
            <div class="c" v-for="item in list">
                <div>{{item}}</div>
            </div>
        </div>
        <div class="b"></div>
    </div>
</template>

<style scoped>
.outer {
    width: 400px;
    height: 600px;
    background-color: #747bff;
    display: grid;
    grid-template-areas: "a" "b";
    grid-template-rows: 1fr 0;
    grid-template-columns: 1fr;
}

.a {
    grid-area: a;
    max-height: 100%;
    overflow-y: scroll;
    display: flex;
    flex-direction: column-reverse;
    background-color: skyblue;
}

.b {
    grid-area: b;
    background-color: antiquewhite;
}

.c {
    color: black;
}
</style>
