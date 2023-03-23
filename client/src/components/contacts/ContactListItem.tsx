import React from "react";

class Props {
    userId: bigint = 0n;
    avatar: string = "";
    remark: string = "";
    nickname: string = "";
}

class State {}

export default class ContactListItem extends React.Component<Props, State> {
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    onClick = async () => {}

    onDoubleClick = async () => {}

    render = (): React.ReactNode => {
        return (
            <div className="contact-list-item" onClick={this.onClick} onDoubleClick={this.onDoubleClick}>
                <img src={this.props.avatar} alt="" className='item-avatar' />
                <div className="item-remark">
                    {
                        this.props.remark
                    }
                </div>
                <div className="item-nickname">
                    <span>
                        {this.props.nickname}
                    </span>
                </div>
            </div>
        )
    }
}