import React, { ReactNode } from 'react';
import { Context, GlobalContext } from '../../context/GlobalContext';
import { HttpClient } from '../../net/http';
import './UserMsgListItem.css';

class Props {
    msg: string = "";
    peerId: bigint = 0n;
    avatar: string = "";
    timestamp: bigint = 0n
    number: number = 0;
    remark: string = "";
}

class State { }

class UserMsgListItem extends React.Component<Props, State> {
    static contextType = GlobalContext;

    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    onClick = async () => {
        let context = this.context as Context;
        context.setCurrentChatPeerId(this.props.peerId);
        let msgList = context.msgMap.get(this.props.peerId);
        await context.setUnread(this.props.peerId, false)
        if (msgList !== undefined) {
            let seqNum = msgList[msgList.length - 1].head.seqNum;
            await HttpClient.put('/message/unread', {
                peer_id: this.props.peerId,
                last_read_seq: seqNum
            }, {}, true);
        }
    }

    render(): ReactNode {
        const date = new Date(Number(this.props.timestamp));
        const hours = date.getHours().toString().padStart(2, '0');
        const minutes = date.getMinutes().toString().padStart(2, '0');
        let time = `${hours}:${minutes}`;
        return (
            <div className="user-msg-list-item" onClick={this.onClick}>
                <img src={this.props.avatar} alt="" className='avatar' />
                <div className="remark">
                    {
                        this.props.remark
                    }
                </div>
                <div className="msg">
                    <span>
                        {this.props.msg}
                    </span>
                </div>
                <div className="timestamp">
                    {
                        time
                    }
                </div>
                <div className="number">
                    {
                        this.props.number > 0 ? (this.props.number > 99 ? <div className='number-0'>99+</div> : <div className='number-0'>{this.props.number}</div>) : ''
                    }
                </div>
            </div>
        )
    }
}

export default UserMsgListItem;