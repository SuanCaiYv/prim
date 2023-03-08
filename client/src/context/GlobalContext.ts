import React from 'react';
import { Msg } from "../entity/msg"

class UserMsgListItemData {
    peerId: bigint
    avatar: string
    remark: string
    text: string
    timestamp: bigint
    unreadNumber: number
    constructor(peerId: bigint, avatar: string, remark: string, text: string, timestamp: bigint, unreadNumber: number) {
        this.peerId = peerId
        this.avatar = avatar
        this.remark = remark
        this.text = text
        this.timestamp = timestamp
        this.unreadNumber = unreadNumber
    }
}

class Context {
    userMsgList: Array<UserMsgListItemData>
    contactList: Array<any>
    userId: bigint
    userAvatar: string
    userNickname: string
    currentChatMsgList: Array<Msg>
    currentChatUserId: bigint
    setUserMsgList: (msg: Msg) => void
    setMsgMap: (msg: Msg) => void
    setContactList: (contactList: Array<any>) => void
    setUserId: (userId: bigint) => void
    setUserAvatar: (userAvatar: string) => void
    setUserNickname: (userNickname: string) => void
    setCurrentChatUserId: (userId: bigint) => void
    constructor() {
        this.userMsgList = []
        this.contactList = []
        this.userId = BigInt(0)
        this.userAvatar = ""
        this.userNickname = ""
        this.currentChatMsgList = []
        this.currentChatUserId = BigInt(0)
        this.setUserMsgList = (msg: Msg) => {}
        this.setMsgMap = (msg: Msg) => {}
        this.setContactList = (contactList: Array<any>) => {}
        this.setUserId = (userId: bigint) => {}
        this.setUserAvatar = (userAvatar: string) => {}
        this.setUserNickname = (userNickname: string) => {}
        this.setCurrentChatUserId = (userId: bigint) => {}
    }
}

const GlobalContext = React.createContext(new Context())

export { GlobalContext, Context, UserMsgListItemData }