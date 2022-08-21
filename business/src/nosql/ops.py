import aioredis

from config.appconfig import setup

config = setup()
redis_host = config['redis_host']
redis_port = config['redis_port']
redis_db = config['redis_db']

redis_ops = None


async def init():
    global redis_ops
    redis_ops = aioredis.from_url(f'redis://{redis_host}:{redis_port}/{redis_db}')


def get_instance():
    return redis_ops
