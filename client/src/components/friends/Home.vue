<script setup lang="ts">
import Layout from './Layout.vue'
import localStorage from '../../util/storage'
import {useRouter} from "vue-router"

const router = useRouter()
</script>

<template>
    <div class="friends">
        <Layout></Layout>
    </div>
</template>

<style scoped>
.friends {
    width: 100%;
    height: 100%;
}
</style>
