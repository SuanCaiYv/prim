<script setup lang="ts">
</script>

<template>
    <div class="input-area">
        <textarea class="input"></textarea>
    </div>
</template>

<style scoped>
.input-area {
    grid-area: input-area;
    background-color: white;
    height: 100%;
    width: 100%;
    overflow-y: scroll;
}

.input {
    width: 100%;
    height: 100%;
    border: none;
    padding: 12px 0 0 12px;
    outline: none;
    resize: none;
    box-sizing: border-box;
    border-top: 1px solid gainsboro;
    font-size: 1rem;
    color: black;
    background-color: white;
}

::-webkit-scrollbar {
    display: none;
}
</style>
