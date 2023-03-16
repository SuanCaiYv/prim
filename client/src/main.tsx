import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'
import { KVDB } from './service/database'
// @ts-ignore
BigInt.prototype.toJSON = function () {
    return this.toString()
}

// @ts-ignore
Map.prototype.toJSON = function () {
    return Object.fromEntries(this);
}

let map1 = new Map<string, string>();
map1.set('11', '11');
map1.set('22', '22');
map1.set('33', '33');
await KVDB.set("map1", map1);
let mp1 = await KVDB.get("map1");
console.log(mp1);

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    // <React.StrictMode>
    //   <App />
    // </React.StrictMode>
    <App></App>
)
