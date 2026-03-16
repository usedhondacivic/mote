import asyncio
import colorsys
import math

import rerun as rr

from mote_link.link import MoteClient, Ping, Pong, RequestNetworkScan, Scan, State


def _log_scan(scan: Scan):
    positions = [
        [
            math.cos(p.angle_rads) * p.distance_mm,
            math.sin(p.angle_rads) * p.distance_mm,
        ]
        for p in scan.points
    ]
    colors = []
    for p in scan.points:
        h = (p.distance_mm / 20.0) % 1.0
        r, g, b = colorsys.hsv_to_rgb(h, 1.0, 1.0)
        colors.append([int(r * 255), int(g * 255), int(b * 255)])

    rr.log(
        "lidar_scan",
        rr.Points2D(positions, colors=colors, radii=10.0),
    )


# Example application that connects to Mote and logs sensor data to rerun.
async def run_main():
    rr.init("mote_rerun_example", spawn=True)

    async with MoteClient() as client:
        await client.connect_with_uid("mote-:3")

        print("Pinging Mote")
        await client.send(Ping())

        while True:
            message = await client.recv()
            if message is None:
                continue

            if isinstance(message, Pong):
                print("Got pong from Mote.")
            elif isinstance(message, Ping):
                print("Mote pinged host.")
                await client.send(Pong())
            elif isinstance(message, Scan):
                _log_scan(message)
            elif isinstance(message, State):
                pass  # TODO: log robot state


if __name__ == "__main__":
    asyncio.run(run_main())
