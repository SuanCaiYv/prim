import './Search.css'

function Search() {
    return (
        <div className="search">
            <input className="text-input" type="text" placeholder="Find..." autoComplete='false' autoCorrect='false' autoCapitalize='false'/>
        </div>
    )
}

export default Search;