<script setup lang="ts">
</script>

<template>
    <div class="user-list-item">
        <img class="avatar" src="../../../assets/default-avatar.jpg">
        <div class="remark">小王</div>
        <div class="short-msg">今天吃啥</div>
        <div class="time">17:38</div>
        <div class="count">
            <div class="number">1</div>
        </div>
    </div>
</template>

<style scoped>
.user-list-item {
    height: 60px;
    width: 100%;
    display: grid;
    grid-template-areas:
            "avatar remark time"
            "avatar short-msg count";
    grid-template-rows: 30px 30px;
    grid-template-columns: 60px 1fr 40px;
}

.avatar {
    grid-area: avatar;
    width: calc(100% - 12px);
    height: calc(100% - 12px);
    margin: 6px 6px 6px 6px;
    border-radius: 50%;
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

.short-msg {
    grid-area: short-msg;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 8px;
    font-size: 0.8rem;
    overflow-x: hidden;
    text-align: left;
    line-height: 30px;
    color: black;
}

.time {
    grid-area: time;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 4px;
    font-size: 0.8rem;
    text-align: left;
    line-height: 30px;
    color: black;
}

.count {
    grid-area: count;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 4px;
    font-size: 0.8rem;
    text-align: left;
    line-height: 30px;
    background-color: white;
}

.number {
    height: 14px;
    width: 14px;
    margin: 8px 0 0 16px;
    border-radius: 100%;
    background-color: red;
    font-size: 0.8rem;
    font-weight: bolder;
    text-align: center;
    line-height: 14px;
    color: white;
}
</style>
