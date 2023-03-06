import './Layout.css'

function Layout(props: any) {
    return (
        <div className="layout">
            {props.children}
        </div>
    )
}

export default Layout