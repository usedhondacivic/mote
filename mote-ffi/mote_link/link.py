# PyO3 does not support exporting type stubs for generated modules
# https://github.com/PyO3/maturin/pull/2940
import mote_link.mote_ffi  # ty:ignore[unresolved-import]

from zeroconf.asyncio import AsyncZeroconf
from zeroconf import Zeroconf, IPVersion, AddressResolver

import asyncio
import ipaddress

import socket


async def chose_from_mdns_service(service_name):
    return None


class MoteClient:
    def __init__(self):
        """
        Create a new Mote client.
        """
        self.uid = None
        self.ip = None

    async def __aenter__(self):
        return self

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
            self.ip = socket.gethostbyname(hostname)
            print(f"Resolved IP address for {hostname}: {self.ip}")
        except socket.error as e:
            print(f"Error resolving {hostname}: {e}")
            print(
                "Did you use the correct uid? Is Mote connected to your network? Does your network support mdns?"
            )

    async def connect_with_ip(self, ip: ipaddress.IPv4Address):
        """
        Connect to Mote.

        Use this method if you know the IP of you robot.
        If your network does not support MDNS you must use this method.
        You can find your robots IP by connecting using USB and visiting [the configuration page](https://empriselab.github.io/mote-core/configuration/).
        """
        self.ip = ip
        pass

    async def send(self):
        pass

    async def recv(self):
        pass

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        pass
