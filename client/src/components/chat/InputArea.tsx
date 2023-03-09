import React from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import { Msg } from "../../entity/msg";
import './InputArea.css';

class Props { }

class State {
    value: string = "";
}

class InputArea extends React.Component<Props, State> {
    static contextType = GlobalContext;

    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            let value = this.state.value
            if (value.length > 0) {
                if (value.endsWith('\n')) {
                    value = value.substring(0, value.length - 1);
                }
                let context = this.context as Context;
                let msg = Msg.text(context.userId, context.currentChatPeerId, context.nodeId, value);
                context.sendMsg(msg);
                this.setState({ value: "" });
            }
        }
    }

    onChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        this.setState({ value: e.target.value });
    }

    render(): React.ReactNode {
        return (
            <div className="input-area">
                <textarea className="text-area" value={this.state.value} onChange={this.onChange} onKeyDown={this.handleKeyDown}></textarea>
            </div>
        )
    }
}

export default InputArea