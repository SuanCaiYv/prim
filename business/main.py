import asyncio

import numpy
from flask import Flask

from api.user import user_router
from db.pgsql import init_db
from entity.net import Head, Type, Msg
from net.api import get_net_api
from nosql import ops

app = Flask(__name__)
app.register_blueprint(user_router)


async def f(func):
    print('run')
    msg = await func()
    print(msg)


async def main():
    await init_db()
    # 处理网络连接
    net_api = await get_net_api()
    head = Head(length=numpy.uint16(12), type=Type.Auth, sender=numpy.uint64(1), receiver=numpy.uint64(2),
                timestamp=numpy.uint64(3), seq_num=numpy.uint64(4), version=numpy.uint16(5))
    body = bytearray(b'0x1234567890')
    msg = Msg.from_bytes(head.to_bytes() + body)
    await net_api.send(msg)
    msg = await net_api.recv()
    # 处理redis
    await ops.init()
    app.run(host="127.0.0.1", port=5000)


if __name__ == '__main__':
    asyncio.run(main())
