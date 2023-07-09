import { BrowserRouter, Route, Routes } from 'react-router-dom'
import './App.css'
import ChatMain from './components/chat/Main'
import SignMain from './components/sign/Sign'
import { GlobalContext } from './context/GlobalContext'
import { useEffect, useRef, useState } from 'react'
import { UserMsgListItemData } from './entity/inner'
import { GROUP_ID_THRESHOLD, Msg, Type } from './entity/msg'
import { KVDB, MsgDB } from './service/database'
import { UserInfo } from './service/user/userInfo'
import { HttpClient } from './net/http'
import { buffer2Array, timestamp } from './util/base'
import TestMain from './components/test/Test'
import Contacts from './components/contacts/Contacts'
import More from './components/more/More'
import { invoke } from '@tauri-apps/api'
import { appWindow } from '@tauri-apps/api/window'

function App() {
    let [userMsgListRender, setUserMsgListRender] = useState<Array<UserMsgListItemData>>([]);
    let [userIdRender, setUserIdRender] = useState(10n);
    let [currentChatMsgListRender, setCurrentChatMsgListRender] = useState<Array<Msg>>([]);
    let [currentChatPeerIdRender, setCurrentChatPeerIdRender] = useState(0n);
    let [unAckSetRender, setUnAckSetRender] = useState(new Set<string>());
    let [currentContactUserIdRender, setCurrentContactUserIdRender] = useState(0n);

    let userMsgList = useRef<Array<UserMsgListItemData>>(new Array<UserMsgListItemData>);
    let msgMap = useRef<Map<bigint, Array<Msg>>>(new Map<bigint, Array<Msg>>());
    let userId = useRef<bigint>(0n);
    let currentChatMsgList = useRef<Array<Msg>>(new Array<Msg>());
    let currentChatPeerId = useRef<bigint>(0n);
    let unAckSet = useRef<Set<string>>(new Set<string>());
    let ackSet = useRef<Set<string>>(new Set<string>());
    let currentContactUserId = useRef<bigint>(0n);

    let signNavigate: () => void = () => { };

    const getPeerId = (id1: bigint, id2: bigint): bigint => {
        if (userId.current === id1) {
            return id2;
        } else {
            return id1;
        }
    }

    const clearState = () => {
        userMsgList.current = [];
        setUserMsgListRender([]);
        msgMap.current = new Map<bigint, Array<Msg>>();
        userId.current = 0n;
        setUserIdRender(0n);
        currentChatMsgList.current = [];
        setCurrentChatMsgListRender([]);
        currentChatPeerId.current = 0n;
        setCurrentChatPeerIdRender(0n);
        unAckSet.current = new Set<string>();
        setUnAckSetRender(new Set<string>());
        ackSet.current = new Set<string>();
        currentContactUserId.current = 0n;
        setCurrentContactUserIdRender(0n);
    }

    const pushUserMsgList = async (msg: Msg) => {
        let peerId: bigint;
        if (msg.head.sender >= GROUP_ID_THRESHOLD) {
            peerId = msg.head.receiver;
        } else {
            peerId = getPeerId(msg.head.sender, msg.head.receiver);
        }
        let text = msg.payloadText();
        let timestamp = msg.head.timestamp;
        let [avatar, remark] = await UserInfo.avatarRemark(userId.current, peerId);
        let number = 0;
        let newList: Array<UserMsgListItemData>;
        let item = userMsgList.current.find((item) => {
            return item.peerId === peerId;
        });
        if (msg.head.type === Type.Ack) {
            if (item !== undefined) {
                number = item.unreadNumber;
                newList = [new UserMsgListItemData(peerId, avatar, remark, item.preview, timestamp, number, msg.head.type,
                    buffer2Array(msg.payload), buffer2Array(msg.extension)), ...userMsgList.current.filter((item) => {
                        return item.peerId !== peerId;
                    })]
            } else {
                newList = userMsgList.current;
            }
        } else {
            if (item !== undefined) {
                if (msg.head.timestamp > item.timestamp) {
                    if (msg.head.sender === peerId) {
                        number = item.unreadNumber + 1;
                    } else {
                        number = item.unreadNumber;
                    }
                    newList = [new UserMsgListItemData(peerId, avatar, remark, text, timestamp, number, msg.head.type,
                        buffer2Array(msg.payload), buffer2Array(msg.extension)), ...userMsgList.current.filter((item) => {
                            return item.peerId !== peerId;
                        })]
                } else {
                    newList = userMsgList.current;
                }
            } else {
                if (msg.head.sender === peerId) {
                    number = 1;
                } else {
                    number = 0;
                }
                newList = [new UserMsgListItemData(peerId, avatar, remark, text, timestamp, number, msg.head.type,
                    buffer2Array(msg.payload), buffer2Array(msg.extension)), ...userMsgList.current];
            }
        }
        newList = newList.sort((a, b) => {
            return Number(b.timestamp - a.timestamp);
        });
        userMsgList.current = newList;
        setUserMsgListRender(userMsgList.current);
        await saveUserMsgList();
    }

    const pushMsgMap = async (msg: Msg) => {
        if (msg.head.nodeId === 0 && msg.payloadText() === "") {
            return;
        }
        let peerId;
        if (msg.head.receiver >= GROUP_ID_THRESHOLD) {
            peerId = msg.head.receiver;
        } else {
            peerId = getPeerId(msg.head.sender, msg.head.receiver);
        }
        let list = msgMap.current.get(peerId);
        if (msg.head.type === Type.Ack) {
            let timestamp = BigInt(msg.payloadText())
            if (list !== undefined) {
                for (let i = list.length - 1; i >= 0; --i) {
                    if (list[i].head.sender === msg.head.sender && list[i].head.receiver === msg.head.receiver && list[i].head.timestamp === timestamp) {
                        list[i].head.timestamp = msg.head.timestamp;
                        list[i].head.seqNum = msg.head.seqNum;
                        await saveMsg(list[i]);
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
                msgMap.current.set(peerId, list);
            } else {
                list.push(msg);
            }
            if (msg.head.seqNum !== 0n) {
                await saveMsg(msg);
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
        msgMap.current.set(peerId, newList);
        if (peerId === currentChatPeerId.current) {
            currentChatMsgList.current = newList;
            setCurrentChatMsgListRender(currentChatMsgList.current);
        }
    }

    const setUnSetAckSet = async (msg: Msg) => {
        if (msg.head.nodeId === 0 && msg.payloadText() === "") {
            return;
        }
        if (msg.head.type === Type.Ack) {
            let key = msg.head.sender + "-" + msg.head.receiver + "-" + msg.payloadText();
            if (unAckSet.current.has(key)) {
                unAckSet.current.delete(key);
                setUnAckSetRender(new Set(unAckSet.current));
            } else {
                ackSet.current.add(key);
            }
        } else {
            let key = msg.head.sender + "-" + msg.head.receiver + "-" + msg.head.timestamp;
            if (msg.head.seqNum === 0n) {
                // todo more time for timeout
                setTimeout(async () => {
                    if (ackSet.current.has(key)) {
                        ackSet.current.delete(key);
                        return;
                    }
                    unAckSet.current.add(key);
                    setUnAckSetRender(new Set(unAckSet.current));
                }, 3000)
            }
        }
    }

    const setUserMsgListItemUnread = async (peerId: bigint, unread: boolean) => {
        let newList = userMsgList.current.map((item) => {
            if (item.peerId === peerId) {
                item.unreadNumber = unread ? 1 : 0;
            }
            return item;
        });
        userMsgList.current = newList;
        setUserMsgListRender(userMsgList.current);
        let currMsgList = msgMap.current.get(peerId)
        setTimeout(async () => {
            if (currMsgList !== undefined) {
                for (let i = currMsgList.length - 1; i >= 0; --i) {
                    if (currMsgList[i].head.seqNum !== 0n) {
                        await HttpClient.put('/message/unread', {
                            peer_id: peerId,
                            last_read_seq: unread ? currMsgList[i].head.seqNum - 1n : currMsgList[i].head.seqNum,
                        }, {}, true);
                        break;
                    }
                }
            }
        }, 300);
        await saveUserMsgList();
    }

    const changeCurrentChatPeerId = (peerId: bigint) => {
        let list = msgMap.current.get(peerId)
        if (list === undefined) {
            list = [];
            msgMap.current.set(peerId, list);
        }
        currentChatMsgList.current = list;
        currentChatPeerId.current = peerId;
        setCurrentChatMsgListRender(currentChatMsgList.current);
        setCurrentChatPeerIdRender(currentChatPeerId.current);
        setUserMsgListItemUnread(peerId, false);
    }

    const changeCurrentContactUserId = (userId: bigint) => {
        currentContactUserId.current = userId;
        setCurrentContactUserIdRender(currentContactUserId.current);
    }

    const openNewChat = async (peerId: bigint) => {
        if (msgMap.current.get(peerId) === undefined) {
            msgMap.current.set(peerId, []);
        }
        let temp = userMsgList.current.find((item) => {
            return item.peerId === peerId;
        });
        let newList = userMsgList.current;
        if (temp === undefined) {
            let fromSeqNum = await MsgDB.latestSeqNum(userId.current, peerId);
            let seqNum = fromSeqNum < 100n ? 1n : fromSeqNum - 100n;
            // load msg from local storage
            let localList = await MsgDB.getMsgList(userId.current, peerId, seqNum, fromSeqNum + 1n);
            for (let j = localList.length - 1; j >= 0; --j) {
                await newMsg(localList[j]);
            }
            let resp = await HttpClient.get("/message/unread", {
                peer_id: peerId,
            }, true);
            if (!resp.ok) {
                return;
            }
            let unreadSeqNum = BigInt(resp.data);
            let lastSeqNum = fromSeqNum;
            if (unreadSeqNum <= lastSeqNum) {
                let item = userMsgList.current.find((item) => {
                    return item.peerId === peerId;
                });
                if (item !== undefined) {
                    item.unreadNumber = Number(lastSeqNum - unreadSeqNum);
                    newList = userMsgList.current.filter((item) => {
                        return item.peerId !== peerId;
                    });
                    newList = [item, ...newList];
                }
            }
            if (localList.length === 0) {
                let [avatar, remark] = await UserInfo.avatarRemark(userId.current, peerId);
                let emptyItem = new UserMsgListItemData(peerId, avatar, remark, "", timestamp(), 0, 0, [], []);
                newList = [emptyItem, ...newList];
            }
        }
        userMsgList.current = newList;
        setUserMsgListRender(userMsgList.current);
        currentChatPeerId.current = peerId;
        setCurrentChatPeerIdRender(currentChatPeerId.current);
        await saveUserMsgList();
    }

    const loadMore = async () => {
        let seqNum = 0n;
        let index = 0;
        while (seqNum === 0n && index < currentChatMsgList.current.length) {
            seqNum = currentChatMsgList.current[index++].head.seqNum;
        }
        if (seqNum === 0n) {
            return;
        }
        let seqFrom = seqNum - 100n;
        if (seqFrom < 1n) {
            seqFrom = 1n;
        }
        let list = await MsgDB.getMsgList(userId.current, currentChatPeerId.current, seqFrom, seqNum);
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
            await newMsg(item);
        });
    }

    const updateUnread = async () => {
        let newList = new Array<UserMsgListItemData>();
        for (let i = 0; i < userMsgList.current.length; ++i) {
            let item = userMsgList.current[i];
            let resp = await HttpClient.get("/message/unread", {
                peer_id: item.peerId,
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
                newList.push(item);
                continue;
            }
            let unreadSeqNum = BigInt(resp.data);
            let lastSeqNum = await MsgDB.latestSeqNum(userId.current, item.peerId);
            if (unreadSeqNum <= lastSeqNum) {
                item.unreadNumber = Number(lastSeqNum - unreadSeqNum);
            }
            newList.push(item);
        }
        userMsgList.current = newList;
        setUserMsgListRender(userMsgList.current);
        await saveUserMsgList();
    }

    const removeUserMsgListItem = async (peerId: bigint) => {
        let newList = userMsgList.current.filter((item) => {
            return item.peerId !== peerId;
        });
        userMsgList.current = newList;
        setUserMsgListRender(userMsgList.current);
        await saveUserMsgList();
        currentChatPeerId.current = 0n;
        setCurrentChatPeerIdRender(currentChatPeerId.current);
    }

    const sendMsg = async (msg: Msg) => {
        await newMsg(msg)
        try {
            await invoke("send", {
                params: {
                    raw: [...new Uint8Array(msg.toArrayBuffer())]
                }
            })
        } catch (e) {
            console.log(e);
        }
    }

    const recvMsg = async (msg: Msg) => {
        if (msg.head.receiver === 0n) {
            msg.head.receiver = userId.current;
        }
        // ignore with broadcast msg.
        if (msg.head.sender >= GROUP_ID_THRESHOLD) {
            let realSender = BigInt(msg.extensionText());
            if (realSender === userId.current) {
                return;
            }
        }
        await newMsg(msg);
    }

    const newMsg = async (msg: Msg) => {
        await pushMsgMap(msg);
        await setUnSetAckSet(msg);
        await pushUserMsgList(msg);
    }

    const loadNewMsg = async (msg: Msg) => {
        await pushMsgMap(msg);
        await pushUserMsgList(msg);
    }

    // @ts-ignore
    const checkMsgList = async () => {
        for (let i = 0; i < userMsgList.current.length; ++i) {
            let item = userMsgList.current[i];
            let latestSeqNum = await MsgDB.latestSeqNum(userId.current, item.peerId);
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
                    await newMsg(msg);
                }
                fromSeqNum = BigInt(msgList[msgList.length - 1][0]) + 1n;
                toSeqNum = fromSeqNum + 100n;
            }
        }
    }

    const setup = async () => {
        let token = await KVDB.get("access-token");
        let userId0 = await KVDB.get("user-id");
        if (token === undefined || userId === undefined) {
            signNavigate();
            return;
        } else {
            let resp = await HttpClient.put('/user', {}, {}, true);
            if (!resp.ok) {
                signNavigate();
                return;
            }
        }
        userId.current = BigInt(userId0);
        setUserIdRender(userId.current);
        let resp = (await HttpClient.get("/which_address", {}, true))
        if (!resp.ok) {
            alert("unknown error")
            return;
        }
        let address = resp.data as string;
        // @todo mode switch
        await connect(address, token, "udp", userId.current, 0);
        await inbox();
        await mergeUserMsgList();
        await syncMsgList();
        await updateUnread();
        let [avatar, _nickname] = await UserInfo.avatarNickname(userId.current);
        await KVDB.set("avatar", avatar);
        currentChatPeerId.current = 0n;
        currentContactUserId = userId;
        setCurrentChatPeerIdRender(currentChatPeerId.current);
        setCurrentContactUserIdRender(currentContactUserId.current);
    }

    const inbox = async () => {
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
        userMsgList.current = res;
    }

    const mergeUserMsgList = async () => {
        let obj = await KVDB.get('user-msg-list-' + userId.current);
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
        for (let i = 0; i < userMsgList.current.length; ++i) {
            map.set(userMsgList.current[i].peerId, userMsgList.current[i]);
        }
        let res = new Array<UserMsgListItemData>();
        map.forEach((value: UserMsgListItemData, _key: BigInt) => {
            res.push(value);
        });
        res.sort((a: UserMsgListItemData, b: UserMsgListItemData) => {
            return Number(a.timestamp - b.timestamp);
        });
        userMsgList.current = res;
        setUserMsgListRender(userMsgList.current);
        await saveUserMsgList();
    }

    const syncMsgList = async () => {
        let peerIdList = new Array<bigint>();
        for (let i = 0; i < userMsgList.current.length; ++i) {
            peerIdList.push(userMsgList.current[i].peerId);
        }
        for (let i = 0; i < peerIdList.length; ++i) {
            let peerId = peerIdList[i];
            let latestSeqNum = await MsgDB.latestSeqNum(userId.current, peerId);
            let seqNum = latestSeqNum < 100n ? 1n : latestSeqNum - 100n;
            // load msg from local storage
            let localList = await MsgDB.getMsgList(userId.current, peerId, seqNum, latestSeqNum + 1n);
            for (let j = localList.length - 1; j >= 0; --j) {
                await loadNewMsg(localList[j]);
            }
            // load msg from server
            let fromSeqNum = latestSeqNum + 1n;
            let toSeqNum = 0n;
            let resp = await HttpClient.get("/message/history", {
                peer_id: peerId,
                from_seq_num: fromSeqNum,
                to_seq_num: toSeqNum,
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
                continue;
            }
            let msgList = resp.data as Array<any>;
            for (let j = msgList.length - 1; j >= 0; j--) {
                let arr = msgList[j] as Array<number>;
                let body = new Uint8Array(arr.length);
                for (let i = 0; i < arr.length; ++i) {
                    body[i] = arr[i];
                }
                let msg = Msg.fromArrayBuffer(body.buffer);
                await loadNewMsg(msg);
            }
        }
    }

    const setSignNavigateFn = (fn: () => void) => {
        signNavigate = fn;
    }

    const connect = async (remote: string, token: string, mode: string, userId: bigint, nodeId: number) => {
        try {
            await invoke<void>("connect", {
                params: {
                    address: remote,
                    token: token,
                    mode: mode,
                    user_id: userId,
                    node_id: nodeId,
                }
            })
        } catch (e) {
            console.log(e);
            throw e;
        }
        await appWindow.listen<Array<number>>("recv", async (event) => {
            let body = new Uint8Array(event.payload.length);
            for (let i = 0; i < event.payload.length; ++i) {
                body[i] = event.payload[i];
            }
            let msg = Msg.fromArrayBuffer(body.buffer);
            await recvMsg(msg);
        })
        // setUnListen(unListen);
        console.log("connected to server");
        return;
    }

    const disconnect = async () => {
        // unListen();
        try {
            await invoke("disconnect", {});
            console.log("disconnected from server");
        } catch (e) {
            console.log(e);
            return;
        }
    }

    useEffect(() => {
        return () => {
            disconnect();
        }
    }, []);

    const saveMsg = async (msg: Msg) => {
        if (msg.head.receiver >= GROUP_ID_THRESHOLD) {
            let originSender = msg.head.sender;
            msg.head.sender = msg.head.receiver;
            await MsgDB.saveMsg(msg);
            msg.head.sender = originSender;
        } else {
            await MsgDB.saveMsg(msg);
        }
    }

    const saveUserMsgList = async () => {
        await KVDB.set('user-msg-list-' + userId.current, userMsgList.current);
    }

    return (
        <div id={'app'} data-tauri-drag-region>
            <GlobalContext.Provider value={{
                userMsgList: userMsgListRender,
                userId: userIdRender,
                currentChatMsgList: currentChatMsgListRender,
                currentChatPeerId: currentChatPeerIdRender,
                unAckSet: unAckSetRender,
                currentContactUserId: currentContactUserIdRender,
                setCurrentChatPeerId: changeCurrentChatPeerId,
                sendMsg: sendMsg,
                setUnread: setUserMsgListItemUnread,
                setCurrentContactUserId: changeCurrentContactUserId,
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
                        <Route path='/t' element={<TestMain />}></Route>
                    </Routes>
                </BrowserRouter></GlobalContext.Provider>
        </div>
    )
}

export default App
