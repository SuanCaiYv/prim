import React, { ReactHTML } from 'react';
import { HttpClient } from '../../net/http';
import Portal from './alert/Portal';
import './Add.css'

class Props0 {
    accountIdVal: (val: string) => void = () => { }
    remarkVal: (val: string) => void = () => { }
    ok: number = 0
}

class State0 {
    isOk: number = 0
}

class AddFriend extends React.Component<Props0, State0> {
    constructor(props: any) {
        super(props)
        this.state = new State0()
    }

    onAccountIdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.props.accountIdVal(e.target.value)
    }

    onRemarkChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.props.remarkVal(e.target.value)
    }

    componentDidUpdate(prevProps: Readonly<Props0>, prevState: Readonly<State0>, snapshot?: any): void {
        this.setState({ isOk: this.props.ok })
    }

    render = (): React.ReactNode => {
        return (
            <div className='add-friend'>
                <div className='a-f-account-id'>
                    <input type="text" className='a-f-input' placeholder='AccountID' onChange={this.onAccountIdChange}/>
                </div>
                <div className='a-f-remark'>
                    <input type="text" className='a-f-input' placeholder='Remark' onChange={this.onRemarkChange} autoCorrect='off'/>
                </div>
            </div>
        )
    }
}

class Props { }

class State {
    trigger: boolean = false
    accountId: string = ''
    remark: string = ''
    isOk: number = 0
}

class Add extends React.Component<Props, State> {
    ref: React.RefObject<any>;

    constructor(props: any) {
        super(props)
        this.state = new State()
        this.ref = React.createRef();
    }

    onClick = () => {
        this.setState({ trigger: !this.state.trigger })
    }

    onAccountIdChange = (val: string) => {
        this.setState({ accountId: val })
    }

    onRemarkChange = (val: string) => {
        this.setState({ remark: val })
    }

    onDone = async () => {
        if (this.state.accountId === '' || this.state.remark === '') {
            return;
        }
        let resp = await HttpClient.post('/relationship', {}, {
            peer_id: this.state.accountId,
            remark: this.state.remark
        }, true);
        if (resp.ok) {
            this.setState({ isOk: 1 })
        } else {
            this.setState({ isOk: 2 })
        }
    }

    render = (): React.ReactNode => {
        return (
            <div className="add">
                <img src="/assets/add.png" alt="" className='add-img' onClick={this.onClick} />
                <Portal content={<AddFriend accountIdVal={this.onAccountIdChange} remarkVal={this.onRemarkChange} ok={this.state.isOk}></AddFriend>} done={this.onDone} trigger={this.state.trigger}></Portal>
            </div>
        )
    }
}

export default Add;