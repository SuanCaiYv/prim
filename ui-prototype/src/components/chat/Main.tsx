import { useContext, useEffect, useRef, useState } from "react";
import { GlobalContext } from "../../context/GlobalContext";
import { useNavigate } from "react-router-dom";
import UserMsgListItem from "./UserMsgListItem";
import ChatHeader from "./ChatHeader";
import MsgList from "./MsgList";
import InputArea from "./InputArea";
import Layout from "../Layout";
import Header from "../Header";
import List from "../List";
import Main from "../Main";
import './Main.css'
import UserMsgItemRightClick from "./RightOperator";
import ChatInfo from "./ChatInfo";

export default function ChatMain() {
    let [showInfo, setShowInfo] = useState(false);
    let context = useContext(GlobalContext)
    let listRef = useRef<HTMLDivElement>(null);
    const navigate = useNavigate();

    context.setSignNavigate(() => {
        navigate('/sign')
    });

    useEffect(() => {
        setShowInfo(false);
        return () => { };
    }, [context.userMsgList, context.currentChatPeerId])

    return (
        <div className={'chat'} data-tauri-drag-region>
            <Layout>
                <Header clicked='chat'></Header>
                <List ref={listRef}>
                    {
                        context.userMsgList.map((msg, _index) => {
                            let index = 'p' + msg.peerId.toString() + 't' + msg.timestamp.toString() + 'u' + msg.unreadNumber.toString();
                            return <UserMsgListItem key={index}
                                preview={msg.preview}
                                peerId={msg.peerId}
                                avatar={msg.avatar}
                                timestamp={msg.timestamp}
                                number={msg.unreadNumber}
                                remark={msg.remark}
                                rawType={msg.rawType}
                                rawPayload={msg.rawPayload}
                                rawExtension={msg.rawExtension} />
                        })
                    }
                    <UserMsgItemRightClick ref={listRef} />
                </List>
                <Main>
                    {
                        context.currentChatPeerId === 0n ? (
                            <div></div>
                        ) : (
                            <div className="chat-main">
                                <ChatHeader setShowInfo={setShowInfo}></ChatHeader>
                                <ChatInfo showInfo={showInfo}>
                                    <MsgList></MsgList>
                                    <InputArea></InputArea>
                                </ChatInfo>
                            </div>
                        )
                    }
                </Main>
            </Layout>
        </div>
    )
}