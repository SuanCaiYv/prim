<script setup lang="ts">
import {defineProps, ref} from "vue"
import {useRouter} from "vue-router";
import storage from "../../util/storage";

const router = useRouter();
const props = defineProps({
    avatar: String,
    remark: String,
    userId: Number,
    addFriend: Boolean
});

const chat = () => {
    if (props.addFriend) {
        return
    }
    storage.set("CURRENT_CHAT_USER", props.userId + "")
    router.push("/home")
}
</script>

<template>
    <div class="user-list-item">
        <img class="avatar" src="../../../src/assets/default-avatar-2.jpg" @dblclick="chat">
        <div class="remark">{{ props.remark }}</div>
        <div class="id">{{props.userId}}</div>
        <div class="button reject" v-if="props.addFriend">拒绝</div>
        <div class="button confirm" v-if="props.addFriend">好</div>
    </div>
</template>

<style scoped>
.user-list-item {
    height: 60px;
    width: 100%;
    display: grid;
    grid-template-areas:
            "avatar remark reject"
            "avatar id confirm";
    grid-template-rows: 30px 30px;
    grid-template-columns: 60px 1fr 60px;
}

.avatar {
    grid-area: avatar;
    width: calc(100% - 12px);
    height: calc(100% - 12px);
    margin: 6px 6px 6px 6px;
    border-radius: 50%;
}

.avatar:hover {
    cursor: pointer;
}

.remark {
    grid-area: remark;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 8px;
    font-size: 1.0rem;
    font-weight: bolder;
    text-align: left;
    line-height: 30px;
    color: black;
}

.id {
    grid-area: id;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 8px;
    font-size: 1.0rem;
    overflow-x: hidden;
    text-align: left;
    line-height: 30px;
    color: black;
}

.button {
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    border-radius: 12px;
    border: none;
    text-align: center;
    line-height: 30px;
    font-size: 1rem;
}

.reject {
    grid-area: reject;
    font-weight: bolder;
    width: 100%;
    height: 100%;
}

.confirm {
    grid-area: confirm;
    width: 100%;
    height: 100%;
}

.button:hover {
    cursor: pointer;
}
</style>
