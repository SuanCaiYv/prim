import random
import time
import uuid


def timestamp() -> int:
    return int(round(time.time() * 1000))


def generate_account_id() -> int:
    return random.randint(100000, 1 << 33)


def salt() -> str:
    return uuid.uuid4().hex[:12]
