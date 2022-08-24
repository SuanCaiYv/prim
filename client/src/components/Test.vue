<script setup lang="ts">
import {List} from "../util/list";
import {reactive} from "_vue@3.2.37@vue";

let array = [1, 2, 3, 4, 5];
array.splice(2, 0, 6);

console.log(array);
</script>

<template>
</template>

<style scoped>
</style>
