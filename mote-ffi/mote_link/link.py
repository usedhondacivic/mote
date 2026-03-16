from __future__ import annotations

# PyO3 does not support exporting type stubs for generated modules
# https://github.com/PyO3/maturin/pull/2940
import mote_link.mote_ffi as mote_ffi  # ty:ignore[unresolved-import]

import asyncio
import ipaddress
import json
import socket
from dataclasses import dataclass
from enum import Enum
from typing import Union


UDP_PORT = 7475

# Message types


@dataclass
class LidarPoint:
    quality: int
    angle_rads: float
    distance_mm: float


@dataclass
class NetworkConnection:
    ssid: str
    strength: int  # rssi


class BITResult(Enum):
    Waiting = "Waiting"
    Pass = "Pass"
    Fail = "Fail"


@dataclass
class BIT:
    name: str
    result: BITResult


@dataclass
class BITCollection:
    power: list[BIT]
    wifi: list[BIT]
    lidar: list[BIT]
    imu: list[BIT]
    encoders: list[BIT]


@dataclass
class MoteState:
    uid: str
    ip: str | None
    current_network_connection: str | None
    available_network_connections: list[NetworkConnection]
    built_in_test: BITCollection


@dataclass
class Ping:
    pass


@dataclass
class Pong:
    pass


@dataclass
class RequestNetworkScan:
    pass


@dataclass
class SetNetworkConnectionConfig:
    ssid: str
    password: str


@dataclass
class SetUID:
    uid: str


@dataclass
class Scan:
    points: list[LidarPoint]


@dataclass
class State:
    data: MoteState


# Union of all messages the host can send to Mote
HostMessage = Union[Ping, Pong, RequestNetworkScan, SetNetworkConnectionConfig, SetUID]

# Union of all messages Mote can send to the host
MoteMessage = Union[Ping, Pong, Scan, State]


# Converts mote_ffi json based messages into Python native types
def _serialize_host_message(msg: HostMessage) -> str:
    if isinstance(msg, Ping):
        return json.dumps("Ping")
    if isinstance(msg, Pong):
        return json.dumps("Pong")
    if isinstance(msg, RequestNetworkScan):
        return json.dumps("RequestNetworkScan")
    if isinstance(msg, SetNetworkConnectionConfig):
        return json.dumps(
            {"SetNetworkConnectionConfig": {"ssid": msg.ssid, "password": msg.password}}
        )
    if isinstance(msg, SetUID):
        return json.dumps({"SetUID": {"uid": msg.uid}})
    raise TypeError(f"Unknown host message type: {type(msg)}")


# Python native types into mote_ffi json based messages
def _deserialize_mote_message(data) -> MoteMessage:
    if data == "Ping":
        return Ping()
    if data == "Pong":
        return Pong()
    if isinstance(data, dict):
        if "Scan" in data:
            return Scan(points=[LidarPoint(**p) for p in data["Scan"]])
        if "State" in data:
            s = data["State"]
            return State(
                data=MoteState(
                    uid=s["uid"],
                    ip=s.get("ip"),
                    current_network_connection=s.get("current_network_connection"),
                    available_network_connections=[
                        NetworkConnection(**nc)
                        for nc in s["available_network_connections"]
                    ],
                    built_in_test=BITCollection(
                        **{
                            key: [
                                BIT(name=b["name"], result=BITResult(b["result"]))
                                for b in bits
                            ]
                            for key, bits in s["built_in_test"].items()
                        }
                    ),
                )
            )
    raise ValueError(f"Unknown mote message: {data!r}")


# Prompt the client to chose a robot from all devices advertising the mote-api service
async def chose_from_mdns_service(service_name):
    return None


# Simple protocol for dumping byting onto / reading bytes from a queue
class _MoteProtocol(asyncio.DatagramProtocol):
    def __init__(self):
        self.transport: asyncio.DatagramTransport | None = None
        self._queue: asyncio.Queue[bytes] = asyncio.Queue()

    def connection_made(self, transport):
        self.transport = transport

    def datagram_received(self, data, addr):
        self._queue.put_nowait(data)

    def error_received(self, exc):
        print(f"UDP error: {exc}")

    def connection_lost(self, exc):
        pass


#
class MoteClient:
    def __init__(self):
        """
        Create a new Mote client.
        """
        self.ip = None
        self._protocol: _MoteProtocol | None = None
        self._link: mote_ffi.Link | None = None

    async def __aenter__(self):
        return self

    async def _open_connection(self):
        loop = asyncio.get_event_loop()
        _, self._protocol = await loop.create_datagram_endpoint(
            _MoteProtocol,
            remote_addr=(str(self.ip), UDP_PORT),
        )
        self._link = mote_ffi.Link()

    async def connect(self):
        """
        Connect to Mote.

        This method will open an interactive discovery prompt.
        Use this method if you do not know the ip or unique ID of your robot and your network supports MDNS.
        """
        await chose_from_mdns_service("_mote-api._udp.local.")

    async def connect_with_uid(self, uid: str):
        """
        Connect to Mote.

        Use this method if you know the unique ID of your robot, and your network / OS support MDNS.
        """
        try:
            # This will use the underlying OS resolution mechanism
            hostname = f"{uid}.local"
            print(f"Attempting to connect to {hostname}...")
            self.ip = socket.gethostbyname(hostname)
            print(f"Resolved IP address for {hostname}: {self.ip}")
            await self._open_connection()
        except socket.error as e:
            print(f"Error resolving {hostname}: {e}")
            print(
                "Did you use the correct uid? Is Mote connected to your network? Does your network support mdns? If you know the ip of your robot, try using connect_with_ip."
            )

    async def connect_with_ip(self, ip: ipaddress.IPv4Address):
        """
        Connect to Mote.

        Use this method if you know the IP of you robot.
        If your network does not support MDNS you must use this method.
        You can find your robots IP by connecting using USB and visiting [the configuration page](https://empriselab.github.io/mote-core/configuration/).
        """
        self.ip = ip
        await self._open_connection()

    async def send(self, message: HostMessage):
        """
        Send a message to Mote.
        """
        assert self._link is not None and self._protocol is not None, (
            "Not connected, try calling MoteClient.connect"
        )
        assert self._protocol.transport is not None

        self._link.send(_serialize_host_message(message))
        while True:
            transmit_json = self._link.poll_transmit()
            if transmit_json is None:
                break
            self._protocol.transport.sendto(bytes(json.loads(transmit_json)))

    async def recv(self) -> MoteMessage:
        """
        Receive one message from Mote.

        Suspends until a complete message is decoded, yielding control to the
        event loop between packets.
        """
        assert self._link is not None and self._protocol is not None, (
            "Not connected, try calling MoteClient.connect"
        )

        while True:
            data = await self._protocol._queue.get()
            self._link.handle_receive(json.dumps(list(data)))
            message_json = self._link.poll_receive()
            if message_json is not None:
                return _deserialize_mote_message(json.loads(message_json))

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self._protocol is not None:
            assert self._protocol.transport is not None
            self._protocol.transport.close()
            self._protocol = None
