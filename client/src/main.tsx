import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import { Head, Msg, Type } from './entity/msg'
import './index.css'

const test = () => {
  let msg = Msg.text(1n, 2n, 3, "一只老呆狗");
  console.log(new Uint8Array(msg.toArrayBuffer()));
}

test();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
