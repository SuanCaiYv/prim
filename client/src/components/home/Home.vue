<script setup lang="ts">
import Layout from '../base/Layout.vue'
import localStorage from '../../util/storage'
import {useRouter} from "vue-router"

const router = useRouter()

let authed = localStorage.get('authed')
if (authed === "" || authed === 'false') {
    router.push('/sign')
} else {
    router.push('/')
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