import React from "react";
import './ChatHeader.css';
import { Context, GlobalContext } from "../../context/GlobalContext";

class ChatHeader extends React.Component {
    static contextType = GlobalContext;

    render(): React.ReactNode {
        let context = this.context as Context;
        return (
            <div className="chat-header">
                <div className="chat-header-remark">
                    {
                        context.currentChatPeerRemark
                    }
                </div>
            </div>
        )
    }
}

export default ChatHeader