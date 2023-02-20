import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import { Head, Msg, Type } from './entity/msg'
import './index.css'
import { Client } from './net/core'
import { BlockQueue } from './util/queue'

// @ts-ignore
BigInt.prototype.toJSON = function () {
  return this.toString()
}

const test = () => {
  let msg = Msg.text(1n, 2n, 3, "一只狗");
  // console.log(new Uint8Array(msg.toArrayBuffer()));
}

let client = new Client("[::1]:11122", "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhdWQiOjEyMzEyMzEyMywiZXhwIjoxNjc3NTA0MzM0NzcyLCJpYXQiOjE2NzY4OTk1MzQ3NzIsImlzcyI6IlBSSU0iLCJuYmYiOjE2NzY4OTk1MzQ3NzIsInN1YiI6IiJ9.QVvHSHaio7JWNru-IQjrkl5HFDi5pUOMHZFfknJtEZA", "udp", 123123123n, 1)
await client.connect()
await client.send(Msg.text(1n, 2n, 3, "一只猫"))

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
