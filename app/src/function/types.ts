import {reactive, ref} from "vue";
import {KV} from "../api/frontend/entity";
import {Msg} from "../api/backend/entity";

let userMsgSet = reactive<Map<number, Map<number, number>>>(new Map<number, Map<number, number>>)
// accountId : timestamp，应该只在渲染用户消息列表时使用，除此之外都是set操作
let userMsgList = reactive<Array<KV<number, number>>>(new Array<KV<number, number>>());
// 需要持久化，消息序列号升序，渲染时reverse，同时借助css实现从底向上渲染
let msgChannelMap = reactive<Map<string, Array<Msg>>>(new Map<string, Array<Msg>>());
// 每一个信道当前最新的消息，即上次保存的最新的消息序列号
let msgChannelMapNewest = reactive<Map<string, number>>(new Map<string, number>());
// 下一次需要同步的序列号，默认最大值
let msgChannelMapNext = reactive<Map<string, number>>(new Map<string, number>());
// 确保同步只会被进行一次
let msgChannelMapSynced = reactive<Map<string, boolean>>(new Map<string, boolean>());
// 发送消息缓冲区
let sendMsgChannel = reactive<Array<Msg>>(new Array<Msg>())
let withAccountId = ref<number>(0);

export {userMsgSet, userMsgList, msgChannelMap, msgChannelMapNewest, msgChannelMapNext, msgChannelMapSynced, sendMsgChannel, withAccountId}