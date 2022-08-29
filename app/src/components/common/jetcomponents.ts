import {createApp} from 'vue'
import SearchAlert from "./Add.vue"
import AddFriend from './AddFriend.vue'
import CreateGroup from './CreateGroup.vue'

const addFunc = function () {
    let divElement = document.createElement("div");
    const instance = createApp(SearchAlert, {
        divNode: divElement,
        addFriend: addFriend,
        createGroup: createGroup,
    })
    instance.mount(divElement)
    // @ts-ignore
    document.getElementById("app").appendChild(divElement)
}

const addFriend = () => {
    let divElement = document.createElement("div");
    const instance = createApp(AddFriend, {
        divNode: divElement,
    })
    instance.mount(divElement)
    // @ts-ignore
    document.getElementById("app").appendChild(divElement)
}

const createGroup = () => {
    let divElement = document.createElement("div");
    const instance = createApp(CreateGroup, {
        divNode: divElement,
    })
    instance.mount(divElement)
    // @ts-ignore
    document.getElementById("app").appendChild(divElement)
}

export {addFunc, addFriend, createGroup};