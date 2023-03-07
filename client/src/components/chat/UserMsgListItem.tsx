import './UserMsgListItem.css';

function UserMsgListItem(props: any) {
    const date = new Date(Number(props.timestamp));
    const hours = date.getHours().toString().padStart(2, '0');
    const minutes = date.getMinutes().toString().padStart(2, '0');
    let time = `${hours}:${minutes}`;
    return (
        <div className="user-msg-list-item">
            <img src={props.avatar} alt="" className='avatar' />
            <div className="msg">
                <p>
                    {props.msg}
                </p>
            </div>
            <div className="timestamp">
                {
                    time
                }
            </div>
            <div className="number">
                {
                    props.number > 0 ? <div className='number-0'>{props.number}</div> : ''
                }
            </div>
        </div>
    )
}

export default UserMsgListItem;