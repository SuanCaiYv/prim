import './List.css'

export default function List(props: any) {
    return (
        <div className={'list'}>
            {props.children}
        </div>
    )
}