import './Layout.css'

function Layout(props: any) {
    return (
        <div className="layout" data-tauri-drag-region>
            {props.children}
        </div>
    )
}

export default Layout