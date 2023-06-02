import './Main.css'

export default function Main(props: any) {
    return (
        <div className="main">
            {props.children}
        </div>
    )
}