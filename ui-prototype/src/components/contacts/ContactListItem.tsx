import "./ContactListItem.css"
import { useContext } from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import { useNavigate } from "react-router-dom";

const ContactListItem = (props: {
    userId: bigint,
    avatar: string,
    remark: string,
    nickname: string
}) => {
    let navigate = useNavigate();
    let context = useContext(GlobalContext) as Context;

    const onClick = async () => {
        context.setCurrentContactUserId(props.userId)
    }

    const onDoubleClick = async () => {
        await context.openNewChat(BigInt(props.userId));
        navigate("/");
    }

    return (
        <div className="contact-list-item" onClick={onClick} onDoubleClick={onDoubleClick}>
            <img src={props.avatar} alt="" className='c-l-item-avatar' />
            <div className="c-l-item-remark">
                {
                    props.remark === '' ? props.nickname : props.remark
                }
            </div>
            <div className="c-l-item-nickname">
                <span>
                    {props.nickname}
                </span>
            </div>
        </div>
    )
}

export default ContactListItem;