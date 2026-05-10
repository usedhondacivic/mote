import json

import pytest

from mote_link.link import (
    DriveBaseState,
    IMUMeasurement,
    Ping,
    Pong,
    RequestNetworkScan,
    Scan,
    SetDriveBaseVelocity,
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

    def test_drive_base_command(self):
        msg = SetDriveBaseVelocity(left_velocity_rad=1.5, right_velocity_rad=-0.5)
        data = json.loads(_serialize_host_message(msg))
        assert data == {
            "DriveBaseCommand": {"left_velocity_rad": 1.5, "right_velocity_rad": -0.5}
        }

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
            {"quality": 1, "angle_rad": 0.1, "distance_mm": 10.0},
            {"quality": 2, "angle_rad": 0.2, "distance_mm": 20.0},
        ]
        result = _deserialize_mote_message({"Scan": points})
        assert isinstance(result, Scan)
        assert len(result.points) == 2
        assert result.points[1].distance_mm == 20.0

    def test_drive_base_state(self):
        data = {
            "DriveBaseState": {
                "left": {
                    "effort_percent": 0.5,
                    "velocity_rad_per_s": 1.0,
                    "position_rad": 0.0,
                },
                "right": {
                    "effort_percent": 0.3,
                    "velocity_rad_per_s": 0.8,
                    "position_rad": 0.1,
                },
            }
        }
        result = _deserialize_mote_message(data)
        assert isinstance(result, DriveBaseState)
        assert result.left.effort_percent == 0.5
        assert result.right.velocity_rad_per_s == 0.8

    def test_imu_measurement(self):
        data = {
            "IMUMeasurement": {
                "accel": {"x": 0.1, "y": 0.2, "z": 9.8},
                "gyro": {"x": 0.01, "y": 0.02, "z": 0.03},
            }
        }
        result = _deserialize_mote_message(data)
        assert isinstance(result, IMUMeasurement)
        assert result.accel.z == 9.8
        assert result.gyro.x == 0.01
