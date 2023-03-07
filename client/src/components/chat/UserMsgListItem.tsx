import './UserMsgListItem.css';

function UserMsgListItem(props: any) {
    return (
        <div className="user-msg-list-item">
            <img src={props.avatar} alt="" className='avatar'/>
            <div className="msg">
                <p>
                    {props.msg}
                </p>
            </div>
            <div className="timestamp">{props.time}</div>
            <div className="number">
                {
                    props.number > 0 ? <span>{props.number}</span> : ''
                }
            </div>
        </div>
    )
}

export default UserMsgListItem;