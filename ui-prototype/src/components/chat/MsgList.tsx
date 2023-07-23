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
                context.currentChatMsgList.map((msg, _index) => {
                    let key;
                    if (msg.head.seqnum !== 0n) {
                        key = 's' + msg.head.sender.toString() + 'r' + msg.head.receiver.toString() + 'seq' + msg.head.seqnum.toString() + ""
                    } else {
                        key = 's' + msg.head.sender.toString() + 'r' + msg.head.receiver.toString() + 't' + msg.head.timestamp.toString() + ""
                    }
                    return <MsgListItem key={key} peerId={msg.head.sender} rawMsg={msg} />
                })
            }
        </div>
    )
}

export default MsgList