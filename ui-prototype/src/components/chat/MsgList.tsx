import React from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import MsgListItem from "./MsgListItem";
import './MsgList.css';

const MsgList = () => {
    let context = React.useContext(GlobalContext) as Context;

    return (
        <div className={'msg-list'}>
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