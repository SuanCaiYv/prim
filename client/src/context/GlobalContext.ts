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
    nodeId: number
    currentChatMsgList: Array<Msg>
    currentChatPeerId: bigint
    currentChatPeerRemark: string
    currentChatPeerAvatar: string
    unAckSet: Set<string>
    setContactList: (contactList: Array<any>) => void
    setUserId: (userId: bigint) => void
    setUserAvatar: (userAvatar: string) => void
    setUserNickname: (userNickname: string) => void
    setCurrentChatPeerId: (userId: bigint) => void
    sendMsg: (msg: Msg) => Promise<void>
    setUnread: (peerId: bigint, unread: boolean) => void
    setLoginPageDirect: (f: () => void) => void
    setup: () => Promise<void>
    disconnect: () => Promise<void>
    // setChatPageDirect: (f: () => void) => void
    constructor() {
        this.userMsgList = []
        this.contactList = []
        this.userId = BigInt(0)
        this.userAvatar = ""
        this.userNickname = ""
        this.nodeId = 0
        this.currentChatMsgList = []
        this.currentChatPeerId = BigInt(0)
        this.currentChatPeerRemark = ""
        this.currentChatPeerAvatar = ""
        this.unAckSet = new Set()
        this.setContactList = (contactList: Array<any>) => {}
        this.setUserId = (userId: bigint) => {}
        this.setUserAvatar = (userAvatar: string) => {}
        this.setUserNickname = (userNickname: string) => {}
        this.setCurrentChatPeerId = (userId: bigint) => {}
        this.sendMsg = async (msg: Msg) => {}
        this.setUnread = (peerId: bigint, unread: boolean) => {}
        this.setLoginPageDirect = () => {}
        this.setup = async () => {}
        this.disconnect = async () => {}
        // this.setChatPageDirect = () => {}
    }
}

const GlobalContext = React.createContext(new Context())

export { GlobalContext, Context, UserMsgListItemData }