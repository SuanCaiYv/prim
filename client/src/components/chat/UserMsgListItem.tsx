import React, { ReactNode } from 'react';
import './UserMsgListItem.css';

class Props {
    msg: string = "";
    peerId: bigint = 0n;
    avatar: string = "";
    timestamp: bigint = 0n
    number: number = 0;
    remark: string = "";
    onClick: (peerId: bigint) => void = (peerId: bigint) => { console.log(peerId) };
}

class State {}

class UserMsgListItem extends React.Component<Props, State> {
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    onClick = () => {
        this.props.onClick(this.props.peerId);
        // todo unread http call
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