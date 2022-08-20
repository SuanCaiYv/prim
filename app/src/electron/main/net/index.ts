import net from 'node:net'

let client: net.Socket;

const connectToServer = function () {
    client = net.createConnection(8190, "127.0.0.1", function () {
        console.log('connected')
    })
}

const onData = function (callback: Function) {
    client.on('data', function (data) {
        callback(data)
    })
}

const onClose = function (callback: Function) {
    client.on('close', function () {
        callback()
    })
}

export {connectToServer, onData, onClose}