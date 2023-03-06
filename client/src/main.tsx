import React from 'react'
import ReactDOM from 'react-dom/client'
import Body from './Body'
import './index.css'
// @ts-ignore
BigInt.prototype.toJSON = function () {
  return this.toString()
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <Body />
  </React.StrictMode>
)
