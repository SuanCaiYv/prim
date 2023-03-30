import "./ContactListItem.css"
import React from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import { Link } from "react-router-dom";

class Props {
    userId: bigint = 0n;
    avatar: string = "";
    remark: string = "";
    nickname: string = "";
}

class State {}

export default class ContactListItem extends React.Component<Props, State> {
    static contextType = GlobalContext;

    chatARef: React.RefObject<any>;
    constructor(props: any) {
        super(props);
        this.state = new State();
        this.chatARef = React.createRef();
    }

    onClick = async () => {
        let context = this.context as Context;
        await context.setCurrentContactUserId(this.props.userId)
    }

    onDoubleClick = async () => {
        let context = this.context as Context;
        await context.openNewChat(BigInt(this.props.userId));
        this.chatARef.current.click();
    }

    render = (): React.ReactNode => {
        return (
            <div className="contact-list-item" onClick={this.onClick} onDoubleClick={this.onDoubleClick}>
                <img src={this.props.avatar} alt="" className='c-l-item-avatar' />
                <div className="c-l-item-remark">
                    {
                        this.props.remark === '' ? this.props.nickname : this.props.remark
                    }
                </div>
                <div className="c-l-item-nickname">
                    <span>
                        {this.props.nickname}
                    </span>
                </div>
                <Link className="chat-a-direct" to="/" ref={this.chatARef}></Link>
            </div>
        )
    }
}