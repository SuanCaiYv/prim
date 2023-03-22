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
    userMsgList: Array<UserMsgListItemData> = []
    msgMap: Map<bigint, Msg[]> = new Map()
    contactList: Array<any> = []
    userId: bigint = 0n
    nodeId: number = 0
    currentChatMsgList: Array<Msg> = []
    currentChatPeerId: bigint = 0n
    unAckSet: Set<string> = new Set()
    setContactList: (contactList: Array<any>) => void = () => { }
    setUserId: (userId: bigint) => void = () => { }
    setCurrentChatPeerId: (userId: bigint) => void = () => { }
    sendMsg: (msg: Msg) => Promise<void> = async () => {}
    setUnread: (peerId: bigint, unread: boolean) => Promise<void> = () => Promise.resolve();
    setLoginPageDirect: (f: () => void) => void = () => { }
    setup: () => Promise<void> = () => Promise.resolve();
    disconnect: () => Promise<void> = () => Promise.resolve();
    clearState: () => void = () => { }
    loadMore: () => Promise<void> = () => Promise.resolve();
    // setChatPageDirect: (f: () => void) => void
    constructor() {
    }
}

const GlobalContext = React.createContext(new Context())

export { GlobalContext, Context, UserMsgListItemData }