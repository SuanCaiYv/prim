import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'
// @ts-ignore
BigInt.prototype.toJSON = function () {
  return this.toString()
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  // <React.StrictMode>
  //   <App />
  // </React.StrictMode>
  <App />
)
