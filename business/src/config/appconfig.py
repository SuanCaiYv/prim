config = None


def setup() -> dict:
    global config
    if config is None:
        config = dict()
        config['pg_user'] = 'qm'
        config['pg_password'] = 'sm.123456'
        config['pg_host'] = '127.0.0.1'
        config['pg_port'] = '5432'
        config['pg_db'] = 'qm'
        config['redis_host'] = '127.0.0.1'
        config['redis_port'] = 6379
        config['redis_db'] = 0
        config['server_host'] = '127.0.0.1'
        config['server_port'] = 8190
    return config
