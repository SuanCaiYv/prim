import React from 'react';
import { UserMsgListItemData } from '../entity/inner';
import { Msg } from "../entity/msg"

class Context {
    userId: bigint = 0n
    userMsgList: Array<UserMsgListItemData> = []
    currentChatMsgList: Array<Msg> = []
    currentChatPeerId: bigint = 0n
    unAckSet: Set<string> = new Set()
    currentContactUserId: bigint = 0n
    setCurrentChatPeerId: (userId: bigint) => void = () => { }
    setCurrentContactUserId: (userId: bigint) => void = () => { }
    sendMsg: (msg: Msg) => Promise<void> = async () => { }
    setUnread: (peerId: bigint, unread: boolean) => Promise<void> = async () => {};
    setup: () => Promise<void> = async () => {};
    disconnect: () => Promise<void> = () => Promise.resolve();
    loadMore: () => Promise<void> = () => Promise.resolve();
    removeUserMsgListItem: (peerId: bigint) => Promise<void> = async () => { }
    openNewChat: (peerId: bigint) => Promise<void> = async () => {}
    clearState: () => void = () => {}
    setSignNavigate: (fn: () => void) => void = () => {}
    constructor() {
    }
}

const GlobalContext = React.createContext(new Context())

export { GlobalContext, Context }