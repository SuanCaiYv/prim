import React from 'react';
import { HttpClient } from '../../net/http';
import './Add.css'
import { alertComponentNormal } from '../portal/Portal';

const Add = () => {
    let [visible, setVisible] = React.useState(false);

    const onClick = () => {
        setVisible(!visible)
    }

    const onAddFriend = () => {
        alertComponentNormal(<AddFriend />)
    }

    const onCreateGroup = () => {
        alertComponentNormal(<CreateGroup />)
    }

    const onInviteMember = () => {
        alertComponentNormal(<InviteMember />)
    }

    const onJoinGroup = () => {
        alertComponentNormal(<JoinGroup />)
    }

    return (
        <div className={'add'} onClick={onClick}>
            <img src="/assets/add.png" alt="" />
            {
                visible ? (
                    <ul className={'add-list'}>
                        <li className={'add-list-item'}>
                            <button onClick={onAddFriend}>AddFriend</button>
                        </li>
                        <li className={'add-list-item'}>
                            <button onClick={onCreateGroup}>CreateGroup</button>
                        </li>
                        <li className={'add-list-item'}>
                            <button onClick={onInviteMember}>InviteMember</button>
                        </li>
                        <li className={'add-list-item'}>
                            <button onClick={onJoinGroup}>JoinGroup</button>
                        </li>
                    </ul>
                ) : null
            }
        </div>
    )
}

const AddFriend = () => {
    let [userId, setUserId] = React.useState('')
    let [remark, setRemark] = React.useState('')
    let [ok, setOk] = React.useState(0)
    let [miss1, setMiss1] = React.useState(false)
    let [miss2, setMiss2] = React.useState(false)

    const onUserIdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setUserId(e.target.value)
    }

    const onRemarkChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setRemark(e.target.value)
    }

    const onClick = async () => {
        if (userId === '') {
            setMiss1(true);
            setTimeout(() => {
                setMiss1(false);
            }, 1000);
            return;
        }
        if (remark === '') {
            setMiss2(true);
            setTimeout(() => {
                setMiss2(false);
            }, 1000);
            return;
        }
        let resp = await HttpClient.post('/relationship', {}, {
            peer_id: BigInt(userId),
            remark: remark
        }, true);
        if (resp.ok) {
            setOk(1);
        } else {
            console.log(resp.errMsg);
            setOk(2);
        }
    }

    return (
        <div className={'add-item'}>
            {
                ok === 0 ? (
                    <div className={'add-item-title'}>
                        NewFriend
                    </div>)
                    : (
                        ok === 1 ? (
                            <div className={'add-item-title-ok'}>
                                NewFriend
                            </div>
                        ) : (
                            <div className={'add-item-title-fail'}>
                                NewFriend
                            </div>
                        )
                    )
            }
            {
                miss1 ? (
                    <div className={'add-item-column1-miss'}>
                        <input type="text" placeholder='AccountID' onChange={onUserIdChange} />
                    </div>
                ) : (
                    <div className={'add-item-column1'}>
                        <input type="text" placeholder='AccountID' onChange={onUserIdChange} />
                    </div>
                )
            }
            {
                miss2 ? (
                    <div className={'add-item-column2-miss'}>
                        <input type="text" placeholder='Remark' onChange={onRemarkChange} autoCorrect='off'/>
                    </div>
                ) : (
                    <div className={'add-item-column2'}>
                        <input type="text" placeholder='Remark' onChange={onRemarkChange} autoCorrect='off'/>
                    </div>
                )
            }
            <div className={'add-item-btn'}>
                <button onClick={onClick}>Add</button>
            </div>
        </div>
    )
}

const CreateGroup = () => {
    let [groupName, setGroupName] = React.useState('')
    let [checkCode, setCheckCode] = React.useState('')
    let [ok, setOk] = React.useState(0)
    let [miss1, setMiss1] = React.useState(false)
    let [miss2, setMiss2] = React.useState(false)

    const onGroupNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setGroupName(e.target.value)
    }

    const onCheckCodeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setCheckCode(e.target.value)
    }

    const onClick = async () => {
        if (groupName === '') {
            setMiss1(true);
            setTimeout(() => {
                setMiss1(false);
            }, 1000);
            return;
        }
        if (checkCode === '') {
            setMiss2(true);
            setTimeout(() => {
                setMiss2(false);
            }, 1000);
            return;
        }
        let resp = await HttpClient.post('/group', {}, {
            group_name: groupName,
            check_code: checkCode
        }, true);
        if (resp.ok) {
            setOk(1);
        } else {
            console.log(resp.errMsg);
            setOk(2);
        }
        setTimeout(() => {
            setOk(0);
        }, 2000);
    }

    return (
        <div className={'add-item'}>
            {
                ok === 0 ? (
                    <div className={'add-item-title'}>
                        CreateGroup
                    </div>)
                    : (
                        ok === 1 ? (
                            <div className={'add-item-title-ok'}>
                                CreateGroup
                            </div>
                        ) : (
                            <div className={'add-item-title-fail'}>
                                CreateGroup
                            </div>
                        )
                    )
            }
            {
                miss1 ? (
                    <div className={'add-item-column1-miss'}>
                        <input type="text" placeholder='GroupName' onChange={onGroupNameChange} autoCorrect='off'/>
                    </div>
                ) : (
                    <div className={'add-item-column1'}>
                        <input type="text" placeholder='GroupName' onChange={onGroupNameChange} autoCorrect='off'/>
                    </div>
                )
            }
            {
                miss2 ? (
                    <div className={'add-item-column2-miss'}>
                        <input type="text" placeholder='CheckCode' onChange={onCheckCodeChange} autoCorrect='off'/>
                    </div>
                ) : (
                    <div className={'add-item-column2'}>
                        <input type="text" placeholder='CheckCode' onChange={onCheckCodeChange} autoCorrect='off'/>
                    </div>
                )
            }
            <div className={'add-item-btn'}>
                <button onClick={onClick}>Create</button>
            </div>
        </div>
    )
}

const InviteMember = () => {
    let [groupId, setGroupId] = React.useState('')
    let [userId, setUserId] = React.useState('')
    let [ok, setOk] = React.useState(0)
    let [miss1, setMiss1] = React.useState(false)
    let [miss2, setMiss2] = React.useState(false)

    const onUserIdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setUserId(e.target.value)
    }

    const onGroupIdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setGroupId(e.target.value)
    }

    const onClick = async () => {
        if (groupId === '') {
            setMiss1(true);
            setTimeout(() => {
                setMiss1(false);
            }, 1000);
            return;
        }
        if (userId === '') {
            setMiss2(true);
            setTimeout(() => {
                setMiss2(false);
            }, 1000);
            return;
        }
        let resp = await HttpClient.post('/group/invite', {}, {
            group_id: BigInt(groupId),
            peer_id: BigInt(userId)
        }, true);
        if (resp.ok) {
            setOk(1);
        } else {
            console.log(resp.errMsg);
            setOk(2);
        }
        setTimeout(() => {
            setOk(0);
        }, 2000);
    }

    return (
        <div className={'add-item'}>
            {
                ok === 0 ? (
                    <div className={'add-item-title'}>
                        InviteMember
                    </div>)
                    : (
                        ok === 1 ? (
                            <div className={'add-item-title-ok'}>
                                InviteMember
                            </div>
                        ) : (
                            <div className={'add-item-title-fail'}>
                                InviteMember
                            </div>
                        )
                    )
            }
            {
                miss1 ? (
                    <div className={'add-item-column1-miss'}>
                        <input type="text" placeholder='GroupID' onChange={onGroupIdChange} />
                    </div>
                ) : (
                    <div className={'add-item-column1'}>
                        <input type="text" placeholder='GroupID' onChange={onGroupIdChange} />
                    </div>
                )
            }
            {
                miss2 ? (
                    <div className={'add-item-column2-miss'}>
                        <input type="text" placeholder='AccountID' onChange={onUserIdChange} />
                    </div>
                ) : (
                    <div className={'add-item-column2'}>
                        <input type="text" placeholder='AccountID' onChange={onUserIdChange} />
                    </div>
                )
            }
            <div className={'add-item-btn'}>
                <button onClick={onClick}>Invite</button>
            </div>
        </div>
    )
}

const JoinGroup = () => {
    let [groupId, setGroupId] = React.useState('')
    let [checkCode, setCheckCode] = React.useState('')
    let [ok, setOk] = React.useState(0)
    let [miss1, setMiss1] = React.useState(false)
    let [miss2, setMiss2] = React.useState(false)

    const onGroupIdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setGroupId(e.target.value)
    }

    const onCheckCodeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setCheckCode(e.target.value)
    }

    const onClick = async () => {
        if (groupId === '') {
            setMiss1(true);
            setTimeout(() => {
                setMiss1(false);
            }, 1000);
            return;
        }
        if (checkCode === '') {
            setMiss2(true);
            setTimeout(() => {
                setMiss2(false);
            }, 1000);
            return;
        }
        let resp = await HttpClient.post('/group/user', {}, {
            group_id: BigInt(groupId),
            check_code: checkCode
        }, true);
        if (resp.ok) {
            setOk(1);
        } else {
            console.log(resp.errMsg);
            setOk(2);
        }
        setTimeout(() => {
            setOk(0);
        }, 2000);
    }

    return (
        <div className={'add-item'}>
            {
                ok === 0 ? (
                    <div className={'add-item-title'}>
                        JoinGroup
                    </div>)
                    : (
                        ok === 1 ? (
                            <div className={'add-item-title-ok'}>
                                JoinGroup
                            </div>
                        ) : (
                            <div className={'add-item-title-fail'}>
                                JoinGroup
                            </div>
                        )
                    )
            }
            {
                miss1 ? (
                    <div className={'add-item-column1-miss'}>
                        <input type="text" placeholder='GroupID' onChange={onGroupIdChange} />
                    </div>
                ) : (
                    <div className={'add-item-column1'}>
                        <input type="text" placeholder='GroupID' onChange={onGroupIdChange} />
                    </div>
                )
            }
            {
                miss2 ? (
                    <div className={'add-item-column2-miss'}>
                        <input type="text" placeholder='CheckCode' onChange={onCheckCodeChange} autoCorrect='off'/>
                    </div>
                ) : (
                    <div className={'add-item-column2'}>
                        <input type="text" placeholder='CheckCode' onChange={onCheckCodeChange} autoCorrect='off'/>
                    </div>
                )
            }
            <div className={'add-item-btn'}>
                <button className={'add-button'} onClick={onClick}>Join</button>
            </div>
        </div>
    )
}

export default Add;