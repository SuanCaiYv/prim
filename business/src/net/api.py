import asyncio

from config.appconfig import setup
from entity.net import HEAD_LENGTH, Msg, Head

config = setup()

net = None


class Net:
    writer: asyncio.StreamWriter
    reader: asyncio.StreamReader

    async def run(self):
        (self.reader, self.writer) = await asyncio.open_connection(config['server_host'], config['server_port'])

    async def send(self, msg: Msg):
        arr = msg.to_bytes()
        self.writer.write(arr)
        await self.writer.drain()

    async def recv(self):
        head_array = await self.reader.read(HEAD_LENGTH)
        head = Head.from_bytes(content=head_array)
        body_array = await self.reader.read(head.length)
        msg = Msg.from_bytes(content=(bytearray(head_array) + bytearray(body_array)))
        return msg

    def recv_async(self, callback):
        asyncio.get_running_loop().create_task(callback(self.recv()))

    @staticmethod
    async def callback_example(get_msg):
        msg = await get_msg()
        print(msg)


async def get_net_api():
    global net
    if net is None:
        net = Net()
        await net.run()
    return net
