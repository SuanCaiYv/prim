import asyncio

from config.appconfig import setup
from entity.net import HEAD_LENGTH, Msg, Head

config = setup()


class Net:
    writer: asyncio.StreamWriter
    reader: asyncio.StreamReader

    async def run(self):
        (self.writer, self.reader) = asyncio.open_connection(config['server_host'], config['server_port'])

    async def send(self, msg: Msg):
        self.writer.write(msg.to_bytes())

    async def recv(self):
        head = Head.from_bytes(content=(await self.reader.read(HEAD_LENGTH)))
        msg = Msg.from_bytes(content=(await self.reader.read(head.length)))
        return msg

    def recv_async(self, callback):
        asyncio.create_task(callback(self.recv()))

    @staticmethod
    async def callback_example(get_msg):
        msg = await get_msg()
        print(msg)
