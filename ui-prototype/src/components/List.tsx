import React from 'react';
import './List.css'

const List = React.forwardRef<HTMLDivElement, any>((props: any, ref: any) => {
    return (
        <div className={'list'} ref={ref}>
            {props.children}
        </div>
    )
});

export default List;