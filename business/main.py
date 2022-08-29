import asyncio

from numpy import uint64, uint16
from flask import Flask
from flask_cors import CORS

import util.base
from api.user import user_router
from entity.net import Head, Type, Msg
from net.api import get_net_api
from nosql import ops

app = Flask(__name__)
app.register_blueprint(user_router)
CORS(app)


async def main():
    # await init_db()
    # 处理网络连接
    net_api = await get_net_api()
    head = Head(length=uint16(12), type=Type.Auth, sender=uint64(0), receiver=uint64(0),
                timestamp=uint64(util.base.timestamp()), seq_num=uint64(0), version=uint16(0))
    body = bytearray(b'0x1234567890')
    msg = Msg.from_bytes(head.to_bytes() + body)
    await net_api.send(msg)
    await net_api.recv()
    # 处理redis
    await ops.init()
    app.run(host="127.0.0.1", port=8290, threaded=False, processes=10)


if __name__ == '__main__':
    asyncio.run(main())
