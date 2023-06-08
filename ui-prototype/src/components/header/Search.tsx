import './Search.css'

function Search() {
    return (
        <div className={'search'} data-tauri-drag-region>
            <input className="text-input" type="text" placeholder="Find..." autoComplete='off' autoCorrect='off' autoCapitalize='off'/>
        </div>
    )
}

export default Search;