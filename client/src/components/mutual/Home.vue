<script setup lang="ts">
import {useRouter} from "vue-router";
import localStorage from '../../util/storage'

const router = useRouter()

function login() {
    localStorage.set('authed', "true")
    router.push('/home')
}

console.log("sign")
</script>

<template>
    <div class="home">
        <div class="user-id input">
            <div class="prefix">账号</div>
            <input type="text" class="input-box">
        </div>
        <div class="password input">
            <div class="prefix">密码</div>
            <input type="password" class="input-box">
        </div>
        <div class="login button">
            <button class="button-box" @click="login()">登录</button>
        </div>
        <div class="sign button">
            <button class="button-box">注册</button>
        </div>
    </div>
</template>

<style scoped>
.home {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-areas:
        "na3 na1 na1 na4"
        "na3 user-id user-id na4"
        "na3 password password na4"
        "na3 login sign na4"
        "na3 na2 na2 na4";
    grid-template-columns: 1fr 180px 180px 1fr;
    grid-template-rows: 1fr 60px 60px 80px 2fr;
}

.input {
    border: none;
    box-sizing: border-box;
    border-radius: 16px;
    font-size: 1.4rem;
    font-weight: bolder;
    line-height: 40px;
    text-align: left;
}

.user-id {
    grid-area: user-id;
}

.password {
    grid-area: password;
}

.button {
    border: none;
    box-sizing: border-box;
    border-radius: 16px;
    font-size: 1.4rem;
    font-weight: bolder;
    line-height: 40px;
    text-align: center;
}

.login {
    grid-area: login;
}

.sign {
    grid-area: sign;
}

.prefix {
    display: inline-block;
    height: 60px;
    width: 60px;
    box-sizing: border-box;
    text-align: left;
    justify-content: center;
    line-height: 60px;
}

.input-box {
    height: calc(100% - 8px);
    display: inline-block;
    width: calc(100% - 60px);
    padding: 0 0 0 8px;
    margin: 4px 0 4px 0;
    border: 0;
    font-size: 1.4rem;
    box-sizing: border-box;
    border-radius: 16px;
    vertical-align: top;
    background-color: #e7e8e8;
}

.input-box:focus {
    outline: none;
}

.button-box {
    height: calc(100% - 32px);
    display: inline-block;
    width: calc(100% - 60px);
    padding: 0 16px 0 16px;
    margin: 16px 0 16px 0;
    border: 0;
    box-sizing: border-box;
    border-radius: 16px;
    vertical-align: top;
    background-color: white;
}

.button-box:hover {
    background-color: #f7f8f8;
}

.button-box:active {
    background-color: gainsboro;
}

.button-box:focus {
    outline: none;
}
</style>
