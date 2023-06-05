import React from 'react';
import { Context, GlobalContext } from '../../context/GlobalContext';
import './RightOperator.css';
import { useContext, useEffect, useRef, useState } from "react";

interface iStyle {
    position: any,
    left: number,
    top: number
}

const UserMsgItemRightClick = React.forwardRef<HTMLDivElement, {}>((_props, ref) => {
    const [show, setShow] = useState<boolean>(false);
    const [style, setStyle] = useState<iStyle>({
        position: 'fixed', left: 0, top: 0
    });
    let [position, setPosition] = useState<{x: number, y: number}>({x: 0, y: 0})
    let showRef = useRef<Boolean>();
    let rightClickRef = useRef<HTMLDivElement>(null);
    let context = useContext(GlobalContext) as Context;

    const handleContextMenu = (event: any) => {
        let parentRef = ref as React.RefObject<HTMLDivElement>;
        event.preventDefault();
        if (!parentRef.current) return;
        let { clientX, clientY } = event;
        let parentX = parentRef.current.offsetWidth;
        let parentY = parentRef.current.offsetHeight;
        let parentT = parentRef.current.offsetTop;
        if (clientX > parentX || clientY > parentY || clientY < parentT) return;
        setPosition({x: clientX, y: clientY})
        setShow(true);
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

    const unRead = async () => {
        let index = position.y / 60;
        if (index < context.userMsgList.length) {
            let peerId = context.userMsgList[index].peerId;
            context.setUnread(peerId, true);
        }
        setShowFalse();
    }

    const renderContentMenu = () => (
        <div ref={rightClickRef} className={'user-msg-right-click'} style={style} >
            <button className={'u-m-r-c'} onClick={unRead}>
                Mark As Unread
            </button>
            <div className={'u-m-r-c'}>
                Remove
            </div>
            <div className={'u-m-r-c-l'}>
                Clear Chat History
            </div>
        </div>
    );
    return show ? renderContentMenu() : null;
});

export default UserMsgItemRightClick;