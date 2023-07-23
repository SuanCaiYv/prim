import { useContext, useEffect, useState } from 'react';
import './ChatInfo.css'
import { HttpClient } from '../../net/http';
import { Context, GlobalContext } from '../../context/GlobalContext';
import { alertComponentNormal, operationResult } from '../portal/Portal';
import { GROUP_ID_THRESHOLD } from '../../entity/msg';

const SetGroupUser = (props: {
    userId: bigint,
    groupId: bigint,
    isAdmin: boolean,
}) => {
    let [setValue, setSetValue] = useState('SetAsAdminister');

    const onClick1 = () => {
        operationResult(true);
    }

    const onClick2 = () => {
        operationResult(false);
    }

    const onClick3 = () => { }

    const onClick4 = () => { }

    useEffect(() => {
        if (props.isAdmin) {
            setSetValue('SetAsMember')
        }
    }, [])

    return (
        <div className={'c-info-set'}>
            <div className={'c-info-set-btn'}>
                <button onClick={onClick1}>{setValue}</button>
            </div>
            <div className={'c-info-set-btn'}>
                <button onClick={onClick2}>Banishment</button>
            </div>
            <div className={'c-info-set-btn'}>
                <button onClick={onClick3}>RemoveMember</button>
            </div>
            <div className={'c-info-set-btn'}>
                <button onClick={onClick4}>ToDo</button>
            </div>
        </div>
    )
}

const ChatInfo = (props: {
    children: any,
    showInfo: boolean,
}) => {
    let [adminList, setAdminList] = useState<any[]>([])
    let [memberList, setMemberList] = useState<any[]>([])
    let context = useContext(GlobalContext) as Context;

    useEffect(() => {
        if (context.currentChatPeerId < GROUP_ID_THRESHOLD) {
            return;
        }
        HttpClient.get('/group/info/member', {
            group_id: context.currentChatPeerId,
            user_role: 'admin',
            offset: 0,
            limit: 100,
        }, true).then(resp => {
            if (resp.ok) {
                let array = resp.data as any[];
                setAdminList(array)
            }
        });
        HttpClient.get('/group/info/member', {
            group_id: context.currentChatPeerId,
            user_role: 'member',
            offset: 0,
            limit: 1000,
        }, true).then((resp) => {
            if (resp.ok) {
                let array = resp.data as any[];
                setMemberList(array)
            }
        });
    }, [context.currentChatPeerId])

    const onClick = (userId: bigint) => {
        alertComponentNormal(<SetGroupUser userId={userId} isAdmin={userId >= GROUP_ID_THRESHOLD} groupId={context.currentChatPeerId}></SetGroupUser>)
    }

    return (
        <div className="chat-info">
            {
                props.showInfo ? (
                    <div className={'with-info'}>
                        {props.children}
                        <div className={'c-info'}>
                            <p>Administer</p>
                            <ul>
                                {
                                    adminList.map((value, _index) => {
                                        return (
                                            <li id={value.user_id + ''} onClick={() => {
                                                onClick(value.user_id)
                                            }} key={value.user_id}>
                                                <img src="/assets/administer.png" alt="" />
                                                <span>{value.remark}</span>
                                            </li>
                                        )
                                    })
                                }
                            </ul>
                            <p>Member</p>
                            <ul>
                                {
                                    memberList.map((value, _index) => {
                                        return (
                                            <li id={value.user_id + ''} onClick={() => {
                                                onClick(value.user_id)
                                            }} key={value.user_id}>
                                                <img src="/assets/member.png" alt="" />
                                                <span>{value.remark}</span>
                                            </li>
                                        )
                                    })
                                }
                            </ul>
                        </div>
                    </div>
                ) : (
                    <div className={'without-info'}>
                        {props.children}
                    </div>
                )
            }
        </div>
    )
}

export default ChatInfo