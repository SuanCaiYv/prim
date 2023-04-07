import './List.css'

function List(props: any) {
    return (
        <div className="list">
            {props.children}
        </div>
    )
}

export default List;