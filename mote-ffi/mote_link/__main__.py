import asyncio

from mote_link.link import MoteClient


# Example application that moves the robot using the keyboard and logs sensor data to the console.
async def run_main():
    async with MoteClient() as client:
        await client.connect_with_uid("mote-:3")


if __name__ == "__main__":
    asyncio.run(run_main())
