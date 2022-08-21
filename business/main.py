import asyncio

from flask import Flask

from api.user import user_router
from db.pgsql import async_session
from net.api import Net
from nosql import ops

app = Flask(__name__)
app.register_blueprint(user_router)


async def main():
    await ops.init()
    app.run(host="127.0.0.1", port=5000)


if __name__ == '__main__':
    net = Net()
    asyncio.run(main())
