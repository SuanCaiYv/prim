import React from 'react';
import './AddFriend.css'

class Props {
    peerId: bigint = 0n
    nickname: string = ''
    remark: string = ''
}

class State {}

export default class AddFriend extends React.Component<Props, State> {
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    ok = async () => {}

    reject = async () => {}

    render = (): React.ReactNode => {
        return (
            <div className='add-friend-msg'>
                <div className='a-f-m-nickname'>
                    {
                        this.props.nickname
                    }
                </div>
                <div className='a-f-m-remark'>
                    {
                        this.props.remark
                    }
                </div>
                <button className='a-f-m-btn1' onClick={this.ok}>
                    OK
                </button>
                <button className='a-f-m-btn2' onClick={this.reject}>
                    Reject
                </button>
            </div>
        )
    }
}