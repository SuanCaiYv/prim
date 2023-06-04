import { useEffect, useRef, useState } from 'react';
import { alertMin } from '../portal/Portal';
import './Test.css';
import UserMsgItemRightClick from '../chat/RightOperator';

interface iStyle {
    position: any,
    left: number,
    top: number
}

const PublicRightClick = () => {
    const [show, setShow] = useState<boolean>(false);
    const [style, setStyle] = useState<iStyle>({
        position: 'fixed', left: 30, top: 200
    });
    let showRef = useRef<Boolean>();
    const rightClickRef = useRef<HTMLDivElement>(null);

    const handleContextMenu = (event: any) => {
        event.preventDefault();
        setShow(true);
        let { clientX, clientY } = event;
        const screenW: number = window.innerWidth;
        const screenH: number = window.innerHeight;
        let rightClickRefW = 0;
        let rightClickRefH = 0;
        if (rightClickRef.current !== null) {
            rightClickRefW = rightClickRef.current.offsetWidth;
            rightClickRefH = rightClickRef.current.offsetHeight;
        }
        const right = (screenW - clientX) > rightClickRefW;
        const top = (screenH - clientY) > rightClickRefH;
        clientX = right ? clientX + 6 : clientX - rightClickRefW - 6;
        clientY = top ? clientY + 6 : clientY - rightClickRefH - 6;
        setStyle({
            ...style,
            left: clientX,
            top: clientY
        });
    };

    const handleClick = (event: any) => {
        if (!showRef.current) return;
        if (event.target.parentNode !== rightClickRef.current) {
            setShow(false)
        }
    };

    const setShowFalse = () => {
        if (!showRef.current) return;
        setShow(false)
    };

    useEffect(() => {
        document.addEventListener('contextmenu', handleContextMenu);
        document.addEventListener('click', handleClick, true);
        document.addEventListener('scroll', setShowFalse, true);
        return () => {
            document.removeEventListener('contextmenu', handleContextMenu);
            document.removeEventListener('click', handleClick, true);
            document.removeEventListener('scroll', setShowFalse, true);
        }
    }, []);

    useEffect(() => {
        showRef.current = show;
    }, [show]);

    const renderContentMenu = () => (
        <div ref={rightClickRef} className="WeChatContactsAvatarTools" style={style} >
            <div className="rightClickItems">
                Mark as unread
            </div>
            <div className="rightClickItems">
                Mute Notifications
            </div>
            <div className="rightClickItems">
                Remove
            </div>
            <div className="rightClickItems">
                Clear Chat History
            </div>
        </div>
    );
    return show ? renderContentMenu() : null;
};

export default function TestMain() {

    const onClick = () => {
        alertMin('alert test');
    }

    return (
        <div>
            <p>aaaa</p>
            <button onClick={onClick}>aaa</button>
            <p>bbbb</p>
            <p>bbbb</p>
            <p>bbbb</p>
            <p>bbbb</p>
            <p>bbbb</p>
            <p>bbbb</p>
            <UserMsgItemRightClick></UserMsgItemRightClick>
        </div>
    )
}