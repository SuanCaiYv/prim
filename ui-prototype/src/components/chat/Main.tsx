import { useContext, useEffect, useRef } from "react";
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

export default function ChatMain() {
    let context = useContext(GlobalContext)
    const navigate = useNavigate();
    let listRef = useRef<HTMLDivElement>(null);

    context.setSignNavigate(() => {
        navigate('/sign')
    });

    useEffect(() => {
        return () => {};
    }, [])

    return (
        <div className={'chat'}>
            <Layout>
                <Header clicked='chat'></Header>
                <List ref={listRef}>
                    {
                        context.userMsgList.map((msg, index) => {
                            return <UserMsgListItem key={index}
                                preview={msg.preview}
                                peerId={msg.peerId}
                                avatar={msg.avatar}
                                timestamp={msg.timestamp}
                                number={msg.unreadNumber}
                                remark={msg.remark}
                                rawType={msg.rawType}
                                rawPayload={msg.rawPayload}
                                rawExtension={msg.rawExtension}></UserMsgListItem>
                        })
                    }
                    <UserMsgItemRightClick ref={listRef}></UserMsgItemRightClick>
                </List>
                <Main>
                    {
                        context.currentChatPeerId === 0n ? (
                            <div></div>
                        ) : (
                            <div className="chat-main">
                                <ChatHeader></ChatHeader>
                                <MsgList></MsgList>
                                <InputArea></InputArea>
                            </div>
                        )
                    }
                </Main>
            </Layout>
        </div>
    )
}