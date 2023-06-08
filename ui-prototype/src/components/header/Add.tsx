import React from 'react';
import { HttpClient } from '../../net/http';
import Portal from './alert/Portal';
import './Add.css'

class AddFriendProps {
    accountIdVal: (val: string) => void = () => { }
    remarkVal: (val: string) => void = () => { }
    setResCB: (cb: (isOk: number) => void) => void = () => { }
}

class AddFriendState {
    isOk: number = 0
}

class AddFriend extends React.Component<AddFriendProps, AddFriendState> {
    constructor(props: any) {
        super(props)
        this.state = new AddFriendState()
    }

    componentDidMount = (): void => {
        this.props.setResCB(this.cb)
    }

    cb = (isOk: number) => {
        this.setState({ isOk: isOk })
    }

    onAccountIdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.props.accountIdVal(e.target.value)
    }

    onRemarkChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.props.remarkVal(e.target.value)
    }

    render = (): React.ReactNode => {
        return (
            <div className='add-friend'>
                {
                    this.state.isOk === 0 ? (
                        <div className='a-f-title'>
                            AddFriend
                        </div>)
                        : (
                            this.state.isOk === 1 ? (
                                <div className='a-f-title-ok'>
                                    AddFriend
                                </div>
                            ) : (
                                <div className='a-f-title-fail'>
                                    AddFriend
                                </div>
                            )
                        )
                }
                <div className='a-f-account-id'>
                    <input type="text" className='a-f-input' placeholder='AccountID' onChange={this.onAccountIdChange} />
                </div>
                <div className='a-f-remark'>
                    <input type="text" className='a-f-input' placeholder='Remark' onChange={this.onRemarkChange} autoCorrect='off' />
                </div>
            </div>
        )
    }
}

class CreateGroupProps {
    groupNameVal: (val: string) => void = () => { }
    checkCodeVal: (val: string) => void = () => { }
    setResCB: (cb: (isOk: number) => void) => void = () => { }
}

class CreateGroupState {
    isOk: number = 0
}

class CreateGroup extends React.Component<CreateGroupProps, CreateGroupState> {
    constructor(props: any) {
        super(props)
        this.state = new CreateGroupState()
    }

    componentDidMount = (): void => {
        this.props.setResCB(this.cb)
    }

    cb = (isOk: number) => {
        this.setState({ isOk: isOk })
    }

    onGroupNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.props.groupNameVal(e.target.value)
    }

    onCheckCodeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.props.checkCodeVal(e.target.value)
    }

    render = (): React.ReactNode => {
        return (
            <div className='add-friend'>
                {
                    this.state.isOk === 0 ? (
                        <div className='a-f-title'>
                            CreateGroup
                        </div>)
                        : (
                            this.state.isOk === 1 ? (
                                <div className='a-f-title-ok'>
                                    CreateGroup
                                </div>
                            ) : (
                                <div className='a-f-title-fail'>
                                    CreateGroup
                                </div>
                            )
                        )
                }
                <div className='a-f-account-id'>
                    <input type="text" className='a-f-input' placeholder='GroupName' onChange={this.onGroupNameChange} autoCorrect='off' />
                </div>
                <div className='a-f-remark'>
                    <input type="text" className='a-f-input' placeholder='CheckCode' onChange={this.onCheckCodeChange} autoCorrect='off' />
                </div>
            </div>
        )
    }
}

class InviteGroupMemberProps {
    accountIdVal: (val: string) => void = () => { }
    groupIdVal: (val: string) => void = () => { }
    setResCB: (cb: (isOk: number) => void) => void = () => { }
}

class InviteGroupMemberState {
    isOk: number = 0
}

class InviteGroupMember extends React.Component<InviteGroupMemberProps, InviteGroupMemberState> {
    constructor(props: any) {
        super(props)
        this.state = new InviteGroupMemberState()
    }

    componentDidMount = (): void => {
        this.props.setResCB(this.cb)
    }

    cb = (isOk: number) => {
        this.setState({ isOk: isOk })
    }

    onAccountIdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.props.accountIdVal(e.target.value)
    }

    onGroupIdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.props.groupIdVal(e.target.value)
    }

    render = (): React.ReactNode => {
        return (
            <div className='add-friend'>
                {
                    this.state.isOk === 0 ? (
                        <div className='a-f-title'>
                            InviteGroupMember
                        </div>)
                        : (
                            this.state.isOk === 1 ? (
                                <div className='a-f-title-ok'>
                                    InviteGroupMember
                                </div>
                            ) : (
                                <div className='a-f-title-fail'>
                                    InviteGroupMember
                                </div>
                            )
                        )
                }
                <div className='a-f-account-id'>
                    <input type="text" className='a-f-input' placeholder='GroupID' onChange={this.onGroupIdChange} autoCorrect='off' />
                </div>
                <div className='a-f-remark'>
                    <input type="text" className='a-f-input' placeholder='InvitedAccountID' onChange={this.onAccountIdChange} autoCorrect='off' />
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
    groupName: string = ''
    checkCode: string = ''
    inviteGroupMemberGroupId: string = ''
    inviteGroupMemberAccountId: string = ''
    addFriendCB: (isOk: number) => void = () => { }
    createGroupCB: (isOk: number) => void = () => { }
    inviteGroupMemberCB: (isOk: number) => void = () => { }
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

    onGroupNameChange = (val: string) => {
        this.setState({ groupName: val })
    }

    onCheckCodeChange = (val: string) => {
        this.setState({ checkCode: val })
    }

    onInviteGroupMemberGroupIdChange = (val: string) => {
        this.setState({ inviteGroupMemberGroupId: val })
    }

    onInviteGroupMemberAccountIdChange = (val: string) => {
        this.setState({ inviteGroupMemberAccountId: val })
    }

    setAddFriendCB = (cb: (isOk: number) => void) => {
        this.setState({ addFriendCB: cb })
    }

    setInviteGroupMemberCB = (cb: (isOk: number) => void) => {
        this.setState({ inviteGroupMemberCB: cb })
    }

    setCreateGroupCB = (cb: (isOk: number) => void) => {
        this.setState({ createGroupCB: cb })
    }

    onDone = async () => {
        if (this.state.accountId !== '' && this.state.remark !== '') {
            let resp = await HttpClient.post('/relationship', {}, {
                peer_id: Number(this.state.accountId),
                remark: this.state.remark
            }, true);
            if (resp.ok) {
                this.state.addFriendCB(1);
            } else {
                console.log(resp.errMsg);
                this.state.addFriendCB(2);
            }
        } else if (this.state.groupName !== '' && this.state.checkCode !== '') {
            let resp = await HttpClient.post('/group', {}, {
                group_name: this.state.groupName,
                check_code: this.state.checkCode
            }, true);
            if (resp.ok) {
                this.state.createGroupCB(1);
            } else {
                console.log(resp.errMsg);
                this.state.createGroupCB(2);
            }
        } else if (this.state.inviteGroupMemberAccountId !== '' && this.state.inviteGroupMemberGroupId !== '') {
            let resp = await HttpClient.post('/group/invite', {}, {
                group_id: Number(this.state.inviteGroupMemberGroupId),
                peer_id: Number(this.state.inviteGroupMemberAccountId)
            }, true);
            if (resp.ok) {
                this.state.addFriendCB(1);
            } else {
                console.log(resp.errMsg);
                this.state.addFriendCB(2);
            }
        }
    }

    render = (): React.ReactNode => {
        return (
            <div className="add" data-tauri-drag-region>
                <img src="/assets/add.png" alt="" className='add-img' onClick={this.onClick} />
                <Portal contentList={[
                    <AddFriend accountIdVal={this.onAccountIdChange} remarkVal={this.onRemarkChange} setResCB={this.setAddFriendCB}></AddFriend>,
                    <CreateGroup groupNameVal={this.onGroupNameChange} checkCodeVal={this.onCheckCodeChange} setResCB={this.setCreateGroupCB}></CreateGroup>,
                    <InviteGroupMember accountIdVal={this.onInviteGroupMemberAccountIdChange} groupIdVal={this.onInviteGroupMemberGroupIdChange} setResCB={this.setInviteGroupMemberCB}></InviteGroupMember>
                ]} done={this.onDone} trigger={this.state.trigger}></Portal>
            </div>
        )
    }
}

export default Add;