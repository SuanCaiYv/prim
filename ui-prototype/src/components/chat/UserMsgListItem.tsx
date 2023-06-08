import React, { useEffect } from 'react';
import { Context, GlobalContext } from '../../context/GlobalContext';
import { Type } from '../../entity/msg';
import { UserInfo } from '../../service/user/userInfo';
import { array2Buffer } from '../../util/base';
import './UserMsgListItem.css';

const UserMsgListItem = (props: Props) => {
    let [avatar, setAvatar] = React.useState<string>('');
    let [remark, setRemark] = React.useState<string>('');
    let [preview, setPreview] = React.useState<string>('');
    let context = React.useContext(GlobalContext) as Context;

    useEffect(() => {
        (async () => {
            if (props.rawType === Type.AddFriend) {
                let [avatar, nickname] = await UserInfo.avatarNickname(props.peerId);
                let [_, remark] = await UserInfo.avatarRemark(context.userId, props.peerId);
                if (remark !== '') {
                    nickname = remark;
                }
                setAvatar(avatar);
                if (props.rawExtension.length === 0) {
                    setPreview('New Friend Request');
                    setRemark(nickname);
                } else {
                    let res = new TextDecoder().decode(array2Buffer(props.rawExtension));
                    if (res === 'true') {
                        setPreview('We Are Friends Now!');
                        setRemark(nickname);
                    } else {
                        setPreview('Sorry For Rejecting Your Request');
                        setRemark(nickname);
                    }
                }
            } else {
                console.log(props);
                setAvatar(props.avatar);
                setRemark(props.remark);
                setPreview(props.preview);
            }
        })();
    }, []);

    const onClick = async () => {
        context.setCurrentChatPeerId(props.peerId);
    }

    const date = new Date(Number(props.timestamp));
    const hours = date.getHours().toString().padStart(2, '0');
    const minutes = date.getMinutes().toString().padStart(2, '0');
    let time = `${hours}:${minutes}`;
    return (
        <div className={'user-msg-list-item'} onClick={onClick}>
            <div className={'u-m-l-item-avatar'}>
                <img src={avatar} alt="" />
            </div>
            <div className="u-m-l-item-remark">
                {
                    remark
                }
            </div>
            <div className="u-m-l-item-msg">
                <span>
                    {preview}
                </span>
            </div>
            <div className="u-m-l-item-timestamp">
                {
                    time
                }
            </div>
            <div className="u-m-l-item-number">
                {
                    props.number > 0 ? (props.number > 99 ? <div className='number-0'>99+</div> : <div className='number-0'>{props.number}</div>) : ''
                }
            </div>
        </div>
    )
}

class Props {
    preview: string = "";
    peerId: bigint = 0n;
    avatar: string = "";
    timestamp: bigint = 0n
    number: number = 0;
    remark: string = "";
    rawType: Type = Type.Text;
    rawPayload: Array<number> = [];
    rawExtension: Array<number> = [];
}

export default UserMsgListItem;