import './RightOperator.css';
import { useEffect, useRef, useState } from "react";

interface iStyle {
    position: any,
    left: number,
    top: number
}

const UserMsgItemRightClick = () => {
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
        <div ref={rightClickRef} className={'user-msg-right-click'} style={style} >
            <div className={'u-m-r-c'}>
                Mark As Unread
            </div>
            <div className={'u-m-r-c'}>
                Remove
            </div>
            <div className={'u-m-r-c-l'}>
                Clear Chat History
            </div>
        </div>
    );
    return show ? renderContentMenu() : null;
};

export default UserMsgItemRightClick;