import {createApp} from 'vue'
import AlertComponent from "../components/Alert.vue"

const alertFunc = function (alertMsg: string, afterDoneCallback: Function) {
    let divElement = document.createElement("div");
    const instance = createApp(AlertComponent, {
        divNode: divElement,
        msg: alertMsg,
        afterDone: afterDoneCallback,
    })
    instance.mount(divElement)
    // @ts-ignore
    document.getElementById("app").appendChild(divElement)
}

export default alertFunc;