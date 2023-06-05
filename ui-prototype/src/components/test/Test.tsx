import { useEffect, useRef, useState } from 'react';
import { alertMin } from '../portal/Portal';
import './Test.css';

interface iStyle {
    position: any,
    left: number,
    top: number
}

const PublicRightClick = (props: {
    parentRef: React.RefObject<HTMLDivElement>,
}) => {
    const [show, setShow] = useState<boolean>(false);
    const [style, setStyle] = useState<iStyle>({
        position: 'fixed', left: 30, top: 200
    });
    let showRef = useRef<Boolean>();
    const rightClickRef = useRef<HTMLDivElement>(null);

    const handleContextMenu = (event: any) => {
        console.log(props.parentRef.current?.offsetWidth, props.parentRef.current?.offsetHeight);
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
        console.log(clientX, clientY, rightClickRefW, rightClickRefH);
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
        <div ref={rightClickRef} className="" style={style} >
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
    let selfRef = useRef<HTMLDivElement>(null);

    const onClick = () => {
        alertMin('alert test');
    }

    return (
        <div className={'test'} ref={selfRef}>
            <p className={'test1'}>bbbb</p>
            <button className={'test1'} onClick={onClick}>aaa</button>
            <p className={'test1'}>bbbb</p>
            <p className={'test1'}>bbbb</p>
            <p className={'test1'}>bbbb</p>
            <p className={'test1'}>bbbb</p>
            <p className={'test1'}>bbbb</p>
            <p className={'test1'}>bbbb</p>
            <PublicRightClick parentRef={selfRef}></PublicRightClick>
        </div>
    )
}