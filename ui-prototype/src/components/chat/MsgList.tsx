import React, { useEffect } from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import MsgListItem from "./MsgListItem";
import './MsgList.css';

const MsgList = () => {
    let listRef = React.createRef<HTMLDivElement>();
    let context = React.useContext(GlobalContext) as Context;

    useEffect(() => {
        if (listRef.current) {
            listRef.current.scrollTop = listRef.current.scrollHeight
        }
    }, [context.currentChatMsgList])

    return (
        <div className={'msg-list'} ref={listRef}>
            <div className={'load-more'} onClick={() => {
                context.loadMore();
            }}>LoadMore</div>
            {
                context.currentChatMsgList.map((msg, index) => {
                    return <MsgListItem key={index} peerId={msg.head.sender} rawMsg={msg} />
                })
            }
        </div>
    )
}

export default MsgList