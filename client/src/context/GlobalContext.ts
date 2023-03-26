import React from 'react';
import { UserMsgListItemData } from '../entity/inner';
import { Msg } from "../entity/msg"

class Context {
    userId: bigint = 0n
    userMsgList: Array<UserMsgListItemData> = []
    msgMap: Map<bigint, Msg[]> = new Map()
    currentChatMsgList: Array<Msg> = []
    currentChatPeerId: bigint = 0n
    unAckSet: Set<string> = new Set()
    setCurrentChatPeerId: (userId: bigint) => void = () => { }
    sendMsg: (msg: Msg) => Promise<void> = async () => { }
    setUnread: (peerId: bigint, unread: boolean) => Promise<void> = () => Promise.resolve();
    setLoginPageDirect: (f: () => void) => void = () => { }
    setup: () => Promise<void> = () => Promise.resolve();
    disconnect: () => Promise<void> = () => Promise.resolve();
    clearState: () => void = () => { }
    loadMore: () => Promise<void> = () => Promise.resolve();
    removeUserMsgListItem: (peerId: bigint) => Promise<void> = async () => { }
    openNewChat: (peerId: bigint) => Promise<void> = async () => {}
    constructor() {
    }
}

const GlobalContext = React.createContext(new Context())

export { GlobalContext, Context, UserMsgListItemData }