import json

import pytest

from mote_link.link import (
    Ping,
    Pong,
    RequestNetworkScan,
    Scan,
    SetNetworkConnectionConfig,
    SetUID,
    _deserialize_mote_message,
    _serialize_host_message,
)


class TestSerializeHostMessage:
    def test_ping(self):
        assert json.loads(_serialize_host_message(Ping())) == "Ping"

    def test_pong(self):
        assert json.loads(_serialize_host_message(Pong())) == "Pong"

    def test_request_network_scan(self):
        assert (
            json.loads(_serialize_host_message(RequestNetworkScan()))
            == "RequestNetworkScan"
        )

    def test_set_network_connection_config(self):
        msg = SetNetworkConnectionConfig(ssid="MyNetwork", password="secret")
        data = json.loads(_serialize_host_message(msg))
        assert data == {
            "SetNetworkConnectionConfig": {"ssid": "MyNetwork", "password": "secret"}
        }

    def test_set_uid(self):
        data = json.loads(_serialize_host_message(SetUID(uid="mote-123")))
        assert data == {"SetUID": {"uid": "mote-123"}}

    def test_unknown_type_raises(self):
        with pytest.raises(TypeError):
            _serialize_host_message("not_a_message")  # type: ignore[arg-type]


class TestRoundTrip:
    """Serialise a host message to JSON, then deserialise it as a mote message."""

    def test_ping_round_trip(self):
        serialised = _serialize_host_message(Ping())
        result = _deserialize_mote_message(json.loads(serialised))
        assert isinstance(result, Ping)

    def test_pong_round_trip(self):
        serialised = _serialize_host_message(Pong())
        result = _deserialize_mote_message(json.loads(serialised))
        assert isinstance(result, Pong)

    def test_scan_points_preserved(self):
        points = [
            {"quality": 1, "angle_rads": 0.1, "distance_mm": 10.0},
            {"quality": 2, "angle_rads": 0.2, "distance_mm": 20.0},
        ]
        result = _deserialize_mote_message({"Scan": points})
        assert isinstance(result, Scan)
        assert len(result.points) == 2
        assert result.points[1].distance_mm == 20.0
