import './Main.css'

function Main(props: any) {
    return (
        <div className="main">
            {props.children}
        </div>
    )
}

export default Main;