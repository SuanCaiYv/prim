import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import './index.css'

// @ts-ignore
BigInt.prototype.toJSON = function () {
    return this + "n";
}

// @ts-ignore
Map.prototype.toJSON = function () {
    return Object.fromEntries(this);
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <App />
)
