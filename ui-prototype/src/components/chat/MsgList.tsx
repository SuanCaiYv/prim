import React, { useEffect, useState } from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import MsgListItem from "./MsgListItem";
import './MsgList.css';
import { Msg } from "../../entity/msg";

const MsgList = () => {
    let listRef = React.createRef<HTMLDivElement>();
    let context = React.useContext(GlobalContext) as Context;
    let [msgList, setMsgList] = useState<Msg[]>([]);

    useEffect(() => {
        if (listRef.current) {
            listRef.current.scrollTop = listRef.current.scrollHeight
        }
        setMsgList(context.currentChatMsgList);
    }, [context.currentChatMsgList])

    return (
        <div className={'msg-list'} ref={listRef}>
            <div className={'load-more'} onClick={() => {
                context.loadMore();
            }}>LoadMore</div>
            {
                msgList.map((msg, index) => {
                    return <MsgListItem key={index} peerId={msg.head.sender} rawMsg={msg} />
                })
            }
        </div>
    )
}

export default MsgList