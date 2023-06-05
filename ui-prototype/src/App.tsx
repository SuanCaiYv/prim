import { BrowserRouter, Route, Routes } from 'react-router-dom'
import './App.css'
import ChatMain from './components/chat/Main'
import SignMain from './components/sign/Sign'
import { GlobalContext } from './context/GlobalContext'
import { useEffect, useState } from 'react'
import { UserMsgListItemData } from './entity/inner'
import { GROUP_ID_THRESHOLD, Msg, Type } from './entity/msg'
import { KVDB, MsgDB } from './service/database'
import { UserInfo } from './service/user/userInfo'
import { HttpClient } from './net/http'
import { buffer2Array, timestamp } from './util/base'
import { Client } from './net/core'
import TestMain from './components/test/Test'
import Contacts from './components/contacts/Contacts'
import More from './components/more/More'

function App() {
    let [userMsgList, setUserMsgList] = useState<Array<UserMsgListItemData>>([]);
    let [msgMap, setMsgMap] = useState(new Map<bigint, Array<Msg>>());
    let [userId, setUserId] = useState(0n);
    let [currentChatMsgList, setCurrentChatMsgList] = useState<Array<Msg>>([]);
    let [currentChatPeerId, setCurrentChatPeerId] = useState(0n);
    let [unAckSet, setUnAckSet] = useState(new Set<string>());
    let [currentContactUserId, setCurrentContactUserId] = useState(0n);
    let signNavigate: () => void = () => { };

    let netConn: Client | undefined = undefined;

    const getPeerId = (id1: bigint, id2: bigint): bigint => {
        if (userId === id1) {
            return id2;
        } else {
            return id1;
        }
    }

    const _flushState = () => {
        setUserMsgList([]);
        setMsgMap(new Map());
        setUserId(0n);
        setCurrentChatMsgList([]);
        setCurrentChatPeerId(0n);
        setUnAckSet(new Set<string>());
        setCurrentContactUserId(0n);
    }

    const _saveMsg = async (msg: Msg) => {
        await MsgDB.saveMsg(msg);
    }

    const _saveUserMsgList = async (list: Array<UserMsgListItemData>) => {
        await KVDB.set('user-msg-list-' + userId, list);
    }

    const _pushUserMsgList = async (msg: Msg) => {
        if (msg.head.nodeId === 0 && msg.payloadText() === "") {
            return;
        }
        let peerId = getPeerId(msg.head.sender, msg.head.receiver);
        let text = msg.payloadText();
        let timestamp = msg.head.timestamp;
        let [avatar, remark] = await UserInfo.avatarRemark(userId, peerId);
        let number = 0;
        let list = userMsgList;
        let newList
        let item = list.find((item) => {
            return item.peerId === peerId;
        });
        // Ack will trigger resort of user msg list
        if (msg.head.type === Type.Ack) {
            if (item !== undefined) {
                number = item.unreadNumber;
                newList = [new UserMsgListItemData(peerId, avatar, remark, item.preview, timestamp, number, msg.head.type, buffer2Array(msg.payload), buffer2Array(msg.extension)), ...list.filter((item) => {
                    return item.peerId !== peerId;
                })]
            } else {
                newList = list;
            }
        } else {
            if (item !== undefined) {
                if (msg.head.timestamp > item.timestamp) {
                    if (msg.head.sender === peerId) {
                        number = item.unreadNumber + 1;
                    } else {
                        number = item.unreadNumber;
                    }
                    newList = [new UserMsgListItemData(peerId, avatar, remark, text, timestamp, number, msg.head.type, buffer2Array(msg.payload), buffer2Array(msg.extension)), ...list.filter((item) => {
                        return item.peerId !== peerId;
                    })]
                } else {
                    newList = list;
                }
            } else {
                if (msg.head.sender === peerId) {
                    number = 1;
                } else {
                    number = 0;
                }
                newList = [new UserMsgListItemData(peerId, avatar, remark, text, timestamp, number, msg.head.type, buffer2Array(msg.payload), buffer2Array(msg.extension)), ...list];
            }
        }
        newList = newList.sort((a, b) => {
            return Number(b.timestamp - a.timestamp);
        });
        setUserMsgList(newList);
        await _saveUserMsgList(newList);
    }

    const _pushMsgMap = async (msg: Msg) => {
        if (msg.head.nodeId === 0 && msg.payloadText() === "") {
            return;
        }
        let peerId = getPeerId(msg.head.sender, msg.head.receiver);
        let list = msgMap.get(peerId);
        if (msg.head.type === Type.Ack) {
            let timestamp = BigInt(msg.payloadText())
            if (list !== undefined) {
                for (let i = list.length - 1; i >= 0; --i) {
                    if (list[i].head.sender === msg.head.sender && list[i].head.receiver === msg.head.receiver && list[i].head.timestamp === timestamp) {
                        list[i].head.timestamp = msg.head.timestamp;
                        list[i].head.seqNum = msg.head.seqNum;
                        await _saveMsg(list[i]);
                        break;
                    }
                }
            } else {
                return;
            }
        } else {
            if (list === undefined) {
                list = new Array();
                list.push(msg);
                msgMap.set(peerId, list);
            } else {
                list.push(msg);
            }
            if (msg.head.seqNum !== 0n) {
                await _saveMsg(msg);
            }
        }
        let list1 = list.filter((item) => {
            return item.head.seqNum !== 0n;
        });
        let list2 = list.filter((item) => {
            return item.head.seqNum === 0n;
        });
        list1.sort((a, b) => {
            return Number(a.head.seqNum - b.head.seqNum);
        });
        list2.sort((a, b) => {
            return Number(a.head.timestamp - b.head.timestamp);
        });
        let newList = [...list1, ...list2];
        msgMap.set(peerId, newList);
        if (peerId === currentChatPeerId) {
            setCurrentChatMsgList(newList);
        }
    }

    const _setUnSetAckSet = async (msg: Msg) => {
        if (msg.head.nodeId === 0 && msg.payloadText() === "") {
            return;
        }
        if (msg.head.type === Type.Ack) {
            let key = msg.head.sender + "-" + msg.head.receiver + "-" + msg.payloadText();
            unAckSet.delete(key);
            let newSet = new Set(unAckSet);
            setUnAckSet(newSet);
        } else {
            let key = msg.head.sender + "-" + msg.head.receiver + "-" + msg.head.timestamp;
            if (msg.head.seqNum === 0n) {
                setTimeout(async () => {
                    unAckSet.add(key);
                    setUnAckSet(unAckSet);
                }, 3000)
            }
        }
    }

    const _newMsg = async (msg: Msg) => {
        await _pushMsgMap(msg);
        await _setUnSetAckSet(msg);
        await _pushUserMsgList(msg);
    }

    const setUserMsgListItemUnread = async (peerId: bigint, unread: boolean) => {
        let newList = userMsgList.map((item) => {
            if (item.peerId === peerId) {
                item.unreadNumber = unread ? 1 : 0;
            }
            return item;
        });
        setUserMsgList(newList);
        await _saveUserMsgList(newList);
    }

    const updateCurrentChatPeerId = (peerId: bigint) => {
        let list = msgMap.get(peerId)
        if (list === undefined) {
            list = [];
            msgMap.set(peerId, list);
        }
        setCurrentChatMsgList([...list]);
        setCurrentChatPeerId(peerId);
        setUserMsgListItemUnread(peerId, false);
    }

    const removeUserMsgListItem = async (peerId: bigint) => {
        let newList = userMsgList.filter((item) => {
            return item.peerId !== peerId;
        });
        setUserMsgList(newList);
        await _saveUserMsgList(newList);
        setCurrentChatPeerId(0n);
    }

    const openNewChat = async (peerId: bigint) => {
        if (msgMap.get(peerId) === undefined) {
            msgMap.set(peerId, []);
        }
        let list = userMsgList;
        let temp = userMsgList.find((item) => {
            return item.peerId === peerId;
        });
        if (temp === undefined) {
            let fromSeqNum = await MsgDB.latestSeqNum(peerId, userId);
            let seqNum = fromSeqNum < 100n ? 1n : fromSeqNum - 100n;
            // load msg from local storage
            let localList = await MsgDB.getMsgList(peerId, userId, seqNum, fromSeqNum + 1n);
            for (let j = localList.length - 1; j >= 0; --j) {
                await _newMsg(localList[j]);
            }
            list = userMsgList;
            let resp = await HttpClient.get("/message/unread", {
                peer_id: peerId,
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
                return;
            }
            let unreadSeqNum = BigInt(resp.data);
            let lastSeqNum = fromSeqNum;
            if (unreadSeqNum <= lastSeqNum) {
                let item = list.find((item) => {
                    return item.peerId === peerId;
                });
                if (item !== undefined) {
                    item.unreadNumber = Number(lastSeqNum - unreadSeqNum);
                    list = list.filter((item) => {
                        return item.peerId !== peerId;
                    });
                    list = [item, ...list];
                }
            }
            if (localList.length === 0) {
                let [avatar, remark] = await UserInfo.avatarRemark(userId, peerId);
                let emptyItem = new UserMsgListItemData(peerId, avatar, remark, "", timestamp(), 0, 0, [], []);
                list = [emptyItem, ...list];
            }
        }
        setUserMsgList(list);
        await _saveUserMsgList(list);
        setCurrentChatPeerId(peerId);
    }

    const clearState = () => {
        _flushState();
    }

    const sendMsg = async (msg: Msg) => {
        await _newMsg(msg)
        await netConn?.send(msg);
    }

    const recvMsg = async (msg: Msg) => {
        if (msg.head.receiver === 0n) {
            msg.head.receiver = userId;
        }
        if (msg.head.sender >= GROUP_ID_THRESHOLD) {
            let realSender = BigInt(msg.extensionText());
            if (realSender === userId) {
                return;
            }
        }
        await _newMsg(msg);
    }

    const loadMore = async () => {
        let seqNum = 0n;
        let index = 0;
        while (seqNum === 0n && index < currentChatMsgList.length) {
            seqNum = currentChatMsgList[index++].head.seqNum;
        }
        if (seqNum === 0n) {
            return;
        }
        let seqFrom = seqNum - 100n;
        if (seqFrom < 1n) {
            seqFrom = 1n;
        }
        let list = await MsgDB.getMsgList(userId, currentChatPeerId, seqFrom, seqNum);
        if (list.length < 100) {
            if (list.length !== 0) {
                seqNum = list[0].head.seqNum;
            }
            seqFrom = seqNum - (100n - BigInt(list.length));
            if (seqFrom < 1n) {
                seqFrom = 1n;
            }
            let resp = await HttpClient.get("/message/history", {
                peer_id: currentChatPeerId,
                from_seq_num: seqFrom,
                to_seq_num: seqNum,
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
                return;
            }
            let msgList = resp.data as Array<any>;
            msgList.forEach((item) => {
                let arr = item as Array<number>;
                let body = new Uint8Array(arr.length);
                for (let i = 0; i < arr.length; ++i) {
                    body[i] = arr[i];
                }
                let msg = Msg.fromArrayBuffer(body.buffer);
                list.push(msg);
            });
        }
        list.forEach(async (item) => {
            await _newMsg(item);
        });
    }

    const checkMsgList = async () => {
        for (let i = 0; i < userMsgList.length; ++i) {
            let item = userMsgList[i];
            let latestSeqNum = await MsgDB.latestSeqNum(item.peerId, userId);
            let fromSeqNum = latestSeqNum + 1n;
            let toSeqNum = 0n;
            while (true) {
                let resp = await HttpClient.get("/message/history", {
                    peer_id: item.peerId,
                    from_seq_num: fromSeqNum,
                    to_seq_num: toSeqNum,
                }, true);
                if (!resp.ok) {
                    console.log(resp.errMsg);
                    continue;
                }
                let msgList = resp.data as Array<any>;
                if (msgList.length === 0) {
                    break;
                }
                for (let j = 0; j < msgList.length; ++j) {
                    let arr = msgList[j] as Array<number>;
                    let body = new Uint8Array(arr.length);
                    for (let i = 0; i < arr.length; ++i) {
                        body[i] = arr[i];
                    }
                    let msg = Msg.fromArrayBuffer(body.buffer);
                    await _newMsg(msg);
                }
                fromSeqNum = BigInt(msgList[msgList.length - 1][0]) + 1n;
                toSeqNum = fromSeqNum + 100n;
            }
        }
    }

    useEffect(() => {
        console.log('app');
        const setup0 = async () => {
            await setup();
        }
        setup0();
        return () => {
            disconnect();
        }
    }, []);

    const setup = async () => {
        let token = await KVDB.get("access-token");
        let userId = await KVDB.get("user-id");
        if (token === undefined || userId === undefined) {
            signNavigate();
            return;
        } else {
            let resp = await HttpClient.put('/user', {}, {}, true);
            if (!resp.ok) {
                signNavigate();
                return;
            }
            setUserId(BigInt(userId));
        }
        setUserId(BigInt(userId));
        let resp = (await HttpClient.get("/which_address", {}, true))
        if (!resp.ok) {
            alert("unknown error")
            return;
        }
        let address = resp.data as string;
        console.log(address);
        // @todo mode switch
        netConn = new Client(address, token as string, "udp", BigInt(userId), 0, recvMsg);
        let list = await inbox();
        console.log(list);
        list = await mergeUserMsgList(list);
        console.log(list);
        await syncMsgList(list);
        console.log(list);
        await updateUnread();
        await netConn?.connect();
        let [avatar, _nickname] = await UserInfo.avatarNickname(userId);
        await KVDB.set("avatar", avatar);
        setCurrentChatPeerId(0n);
        setCurrentContactUserId(BigInt(userId));
    }

    const inbox = async (): Promise<Array<UserMsgListItemData>> => {
        let inboxResp = await HttpClient.get("/message/inbox", {}, true);
        if (!inboxResp.ok) {
            console.log(inboxResp.errMsg);
            alert("unknown error")
            return Promise.reject();
        }
        let list = inboxResp.data as Array<any>;
        let res = new Array<UserMsgListItemData>();
        for (let i = 0; i < list.length; ++i) {
            let peerId = BigInt(list[i]);
            let userMsgItem = new UserMsgListItemData(peerId, "", "", "", 0n, 0, 0, [], []);
            res.push(userMsgItem);
        }
        return res;
    }

    const mergeUserMsgList = async (inboxList: Array<UserMsgListItemData>): Promise<UserMsgListItemData[]> => {
        let obj = await KVDB.get('user-msg-list-' + userId);
        if (obj === undefined) {
            obj = [];
        }
        let list = new Array<UserMsgListItemData>();
        obj.forEach((value: any) => {
            let item = new UserMsgListItemData(BigInt(value.peerId), value.avatar as string, value.remark as string, value.preview as string, BigInt(value.timestamp), Number(value.unreadNumber), value.rawType, value.rawPayload as Array<number>, value.rawExtension as Array<number>);
            list.push(item);
        });
        let map = new Map<BigInt, UserMsgListItemData>();
        for (let i = 0; i < list.length; ++i) {
            map.set(list[i].peerId, list[i]);
        }
        for (let i = 0; i < inboxList.length; ++i) {
            map.set(inboxList[i].peerId, inboxList[i]);
        }
        let res = new Array<UserMsgListItemData>();
        map.forEach((value: UserMsgListItemData, _key: BigInt) => {
            res.push(value);
        });
        res.sort((a: UserMsgListItemData, b: UserMsgListItemData) => {
            return Number(a.timestamp - b.timestamp);
        });
        setUserMsgList(res);
        setUserMsgList(res);
        await _saveUserMsgList(res);
        return res;
    }

    const syncMsgList = async (list: Array<UserMsgListItemData>) => {
        for (let i = 0; i < list.length; ++i) {
            let item = list[i];
            let latestSeqNum = await MsgDB.latestSeqNum(item.peerId, userId);
            let seqNum = latestSeqNum < 100n ? 1n : latestSeqNum - 100n;
            // load msg from local storage
            let localList = await MsgDB.getMsgList(item.peerId, userId, seqNum, latestSeqNum + 1n);
            for (let j = localList.length - 1; j >= 0; --j) {
                await _newMsg(localList[j]);
            }
            // load msg from server
            let fromSeqNum = latestSeqNum + 1n;
            let toSeqNum = 0n;
            while (true) {
                let resp = await HttpClient.get("/message/history", {
                    peer_id: item.peerId,
                    from_seq_num: fromSeqNum,
                    to_seq_num: toSeqNum,
                }, true);
                if (!resp.ok) {
                    console.log(resp.errMsg);
                    continue;
                }
                let msgList = resp.data as Array<any>;
                if (msgList.length === 0) {
                    break;
                }
                for (let j = 0; j < msgList.length; ++j) {
                    let arr = msgList[j] as Array<number>;
                    let body = new Uint8Array(arr.length);
                    for (let i = 0; i < arr.length; ++i) {
                        body[i] = arr[i];
                    }
                    let msg = Msg.fromArrayBuffer(body.buffer);
                    await _newMsg(msg);
                }
                fromSeqNum = BigInt(msgList[msgList.length - 1][0]) + 1n;
                toSeqNum = fromSeqNum + 100n;
            }
        }
    }

    const updateUnread = async () => {
        let newList = new Array<UserMsgListItemData>();
        for (let i = 0; i < userMsgList.length; ++i) {
            let item = userMsgList[i];
            let resp = await HttpClient.get("/message/unread", {
                peer_id: item.peerId,
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
                newList.push(item);
                continue;
            }
            let unreadSeqNum = BigInt(resp.data);
            let lastSeqNum = await MsgDB.latestSeqNum(item.peerId, userId);
            if (unreadSeqNum <= lastSeqNum) {
                item.unreadNumber = Number(lastSeqNum - unreadSeqNum);
            }
            newList.push(item);
        }
        setUserMsgList(newList);
        await _saveUserMsgList(newList);
    }

    const disconnect = async () => {
        if (netConn !== undefined) {
            await netConn.disconnect();
        }
    }

    const setSignNavigateFn = (fn: () => void) => {
        signNavigate = fn;
    }

    return (
        <div id={'app'}>
            <GlobalContext.Provider value={{
                userMsgList: userMsgList,
                msgMap: msgMap,
                userId: userId,
                currentChatMsgList: currentChatMsgList,
                currentChatPeerId: currentChatPeerId,
                unAckSet: unAckSet,
                currentContactUserId: currentContactUserId,
                setCurrentChatPeerId: updateCurrentChatPeerId,
                sendMsg: sendMsg,
                setUnread: setUserMsgListItemUnread,
                setCurrentContactUserId: setCurrentContactUserId,
                setup: setup,
                disconnect: disconnect,
                loadMore: loadMore,
                removeUserMsgListItem: removeUserMsgListItem,
                openNewChat: openNewChat,
                clearState: clearState,
                setSignNavigate: setSignNavigateFn,
            }}>
                <BrowserRouter>
                    <Routes>
                        <Route path='/' element={<ChatMain></ChatMain>}></Route>
                        <Route path='/sign' element={<SignMain></SignMain>}></Route>
                        <Route path='/contacts' element={<Contacts></Contacts>}></Route>
                        <Route path='/more' element={<More></More>}></Route>
                        <Route path='/t' element={<TestMain/>}></Route>
                    </Routes>
                </BrowserRouter></GlobalContext.Provider>
        </div>
    )
}

export default App
