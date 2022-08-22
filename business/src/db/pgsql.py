import asyncio
from asyncio import current_task

from sqlalchemy.ext.asyncio import create_async_engine, AsyncSession, async_scoped_session
from sqlalchemy.orm import sessionmaker, declarative_base

from config.appconfig import setup

config = setup()
pg_user = config['pg_user']
pg_password = config['pg_password']
pg_host = config['pg_host']
pg_port = config['pg_port']
pg_db = config['pg_db']

engine = create_async_engine(f'postgresql+asyncpg://{pg_user}:{pg_password}@{pg_host}:{pg_port}/{pg_db}', echo=False)
async_session_factory = sessionmaker(engine, expire_on_commit=True, class_=AsyncSession)
async_session = async_scoped_session(async_session_factory, scopefunc=asyncio.current_task)

Base = declarative_base()


async def init_db():
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
