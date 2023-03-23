import React from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import MsgListItem from "./MsgItem";
import './MsgList.css';

class Props { }

class State {
}

class MsgList extends React.Component<Props, State> {
    listRef = React.createRef<HTMLDivElement>();

    static contextType = GlobalContext;

    constructor(props: Props) {
        super(props);
        this.state = new State();
    }

    componentDidMount = async () => {
        if (this.listRef.current) {
            this.listRef.current.scrollTop = this.listRef.current.scrollHeight;
        }
    }

    componentDidUpdate(): void {
        if (this.listRef.current) {
            this.listRef.current.scrollTop = this.listRef.current.scrollHeight;
        }
    }

    render(): React.ReactNode {
        let context = this.context as Context;
        return (
            <div className="msg-list" ref={this.listRef}>
                <div>LoadMore</div>
                {
                    context.currentChatMsgList.map((msg, index) => {
                        return <MsgListItem key={index} content={msg.payloadText()} accountId={msg.head.sender} timestamp={msg.head.timestamp}></MsgListItem>
                    })
                }
            </div>
        )
    }
}

export default MsgList