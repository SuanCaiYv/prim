import './List.css'

function List(props: any) {
    return (
        <div className="list" onClick={props.clearChatArea}>
            {props.children}
        </div>
    )
}

export default List;