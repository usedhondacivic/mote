import asyncio
import colorsys
import math

import rerun as rr
import rerun.blueprint as rrb

from mote_link.link import (
    DriveBaseState,
    IMUMeasurement,
    MoteClient,
    MoteConnectionError,
    Ping,
    Pong,
    Scan,
    SetDriveBaseVelocity,
    State,
)


def _log_drive_base_state(state: DriveBaseState):
    for side, wheel in [("left", state.left), ("right", state.right)]:
        rr.log(f"drive_base/{side}/effort_percent", rr.Scalars(wheel.effort_percent))
        rr.log(
            f"drive_base/{side}/velocity_rad_per_s",
            rr.Scalars(wheel.velocity_rad_per_s),
        )
        rr.log(f"drive_base/{side}/position_rad", rr.Scalars(wheel.postition_rad))


def _log_imu_measurement(imu: IMUMeasurement):
    rr.log("imu/accel/x", rr.Scalars(imu.accel.x))
    rr.log("imu/accel/y", rr.Scalars(imu.accel.y))
    rr.log("imu/accel/z", rr.Scalars(imu.accel.z))
    rr.log("imu/gyro/x", rr.Scalars(imu.gyro.x))
    rr.log("imu/gyro/y", rr.Scalars(imu.gyro.y))
    rr.log("imu/gyro/z", rr.Scalars(imu.gyro.z))


def _log_scan(scan: Scan):
    positions = [
        [
            math.cos(p.angle_rad) * p.distance_mm,
            math.sin(p.angle_rad) * p.distance_mm,
        ]
        for p in scan.points
    ]
    colors = []
    for p in scan.points:
        h = (p.distance_mm / (20.0 * 360.0)) % 1.0
        r, g, b = colorsys.hsv_to_rgb(h, 1.0, 1.0)
        colors.append([int(r * 255), int(g * 255), int(b * 255)])

    rr.log(
        "lidar_scan",
        rr.Points2D(positions, colors=colors, radii=10.0),
    )


# Example application that connects to Mote and logs sensor data to rerun.
async def run_main():
    rr.init("mote_rerun_example_python")
    server_uri = rr.serve_grpc()
    rr.serve_web_viewer(connect_to=server_uri)

    wheel_time_ranges = [
        rrb.VisibleTimeRange(
            "log_time",
            start=rrb.TimeRangeBoundary.cursor_relative(seconds=-10.0),
            end=rrb.TimeRangeBoundary.cursor_relative(seconds=5.0),
        )
    ]

    # (row label, {side: [entity paths]})
    wheel_signal_rows = [
        (
            "Velocity",
            {
                side: [
                    f"+ /drive_base/{side}/velocity_rad_per_s",
                    f"+ /drive_base/{side}/velocity_command_rad_per_s",
                ]
                for side in ("left", "right")
            },
        ),
        (
            "Effort",
            {
                side: [f"+ /drive_base/{side}/effort_percent"]
                for side in ("left", "right")
            },
        ),
        (
            "Position",
            {
                side: [f"+ /drive_base/{side}/position_rad"]
                for side in ("left", "right")
            },
        ),
    ]

    wheel_rows = [
        rrb.Horizontal(
            *[
                rrb.TimeSeriesView(
                    name=f"{side.capitalize()} Wheel {label}",
                    contents=paths,
                    time_ranges=wheel_time_ranges,
                )
                for side, paths in signals.items()
            ]
        )
        for label, signals in wheel_signal_rows
    ]

    blueprint = rrb.Blueprint(
        rrb.Horizontal(
            rrb.Spatial2DView(
                name="LiDAR",
                origin="/lidar_scan",
                visual_bounds=rrb.VisualBounds2D(
                    x_range=[-7000, 7000], y_range=[-7000, 7000]
                ),
                time_ranges=[
                    rrb.VisibleTimeRange(
                        "log_time",
                        start=rrb.TimeRangeBoundary.cursor_relative(seconds=-0.2),
                        end=rrb.TimeRangeBoundary.cursor_relative(),
                    )
                ],
            ),
            rrb.Vertical(
                rrb.TimeSeriesView(name="Accel", origin="/imu/accel"),
                rrb.TimeSeriesView(name="Gyro", origin="/imu/gyro"),
                *wheel_rows,
            ),
        ),
        rrb.SelectionPanel(state="collapsed"),
        rrb.TimePanel(state="collapsed"),
    )
    rr.send_blueprint(blueprint)

    async with MoteClient() as client:
        await client.connect()

        print("Pinging Mote")
        await client.send(Ping())

        async def recv_loop():
            while True:
                message = await client.recv()

                if isinstance(message, Pong):
                    print("Got pong from Mote.")
                elif isinstance(message, Ping):
                    print("Mote pinged host.")
                    await client.send(Pong())
                elif isinstance(message, Scan):
                    _log_scan(message)
                elif isinstance(message, DriveBaseState):
                    _log_drive_base_state(message)
                elif isinstance(message, IMUMeasurement):
                    _log_imu_measurement(message)
                elif isinstance(message, State):
                    print(f"Got system state {message}")

        async def sine_wave_command_loop():
            amplitude = 6.0  # rad/s
            period = 4.0  # seconds
            dt = 0.05  # 20 Hz command rate
            t = 0.0
            while True:
                velocity = amplitude * math.sin(2 * math.pi * t / period)
                await client.send(
                    SetDriveBaseVelocity(
                        left_velocity_rad=velocity,
                        right_velocity_rad=-velocity,
                    )
                )
                rr.log(
                    "drive_base/left/velocity_command_rad_per_s", rr.Scalars(velocity)
                )
                rr.log(
                    "drive_base/right/velocity_command_rad_per_s",
                    rr.Scalars(-velocity),
                )
                t += dt
                await asyncio.sleep(dt)

        await asyncio.gather(recv_loop(), sine_wave_command_loop())


def main():
    try:
        asyncio.run(run_main())
    except KeyboardInterrupt:
        print("\nDisconnected.")
    except MoteConnectionError as e:
        print(f"Connection failed: {e}")
        raise SystemExit(1)


if __name__ == "__main__":
    main()
