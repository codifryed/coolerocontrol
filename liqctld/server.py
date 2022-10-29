#  CoolerControl - monitor and control your cooling and other devices
#  Copyright (c) 2022  Guy Boldon
#  |
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU General Public License as published by
#  the Free Software Foundation, either version 3 of the License, or
#  (at your option) any later version.
#  |
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU General Public License for more details.
#  |
#  You should have received a copy of the GNU General Public License
#  along with this program.  If not, see <https://www.gnu.org/licenses/>.
# ----------------------------------------------------------------------------------------------------------------------

import logging
import os
import signal
from http import HTTPStatus

import uvicorn
from fastapi import FastAPI, Request
from fastapi.responses import ORJSONResponse

from device_service import DeviceService
from models import Handshake, LiquidctlException, LiquidctlError, Statuses, InitRequest

SYSTEMD_SOCKET_FD: int = 3
DEFAULT_PORT: int = 11986  # 11987 is the gui std port
log = logging.getLogger(__name__)
api = FastAPI()
device_service = DeviceService()


@api.exception_handler(LiquidctlException)
async def liquidctl_exception_handler(request: Request, exc: LiquidctlException) -> ORJSONResponse:
    return ORJSONResponse(
        status_code=HTTPStatus.INTERNAL_SERVER_ERROR,
        content=LiquidctlError(message=str(exc))
    )


@api.get("/handshake")
async def handshake():
    log.info("Exchanging handshake")
    return Handshake(shake=True)


@api.get("/devices", response_class=ORJSONResponse)
def get_devices() -> ORJSONResponse:
    devices = device_service.get_devices()
    return ORJSONResponse({"devices": devices})


@api.post("/devices/connect")
def connect_devices():
    device_service.connect_devices()
    return {"connected": True}


@api.put("/devices/{device_id}/legacy690", response_class=ORJSONResponse)
def set_device_as_legacy690(device_id: int) -> ORJSONResponse:
    device = device_service.set_device_as_legacy690(device_id)
    return ORJSONResponse({"device": device})


@api.post("/devices/{device_id}/initialize", response_class=ORJSONResponse)
def init_device(device_id: int, init_request: InitRequest) -> ORJSONResponse:
    init_args = init_request.dict(exclude_none=True)
    status: Statuses = device_service.initialize_device(device_id, init_args)
    return ORJSONResponse({"status": status})


@api.get("/devices/{device_id}/status", response_class=ORJSONResponse)
def get_status(device_id: int) -> ORJSONResponse:
    status: Statuses = device_service.get_status(device_id)
    return ORJSONResponse({"status": status})


@api.post("/devices/disconnect")
def disconnect_all():
    """Not necessary to call this explicitely, as /quit will also call disconnect automatically"""
    device_service.disconnect_all()
    return {"disconnected": True}


@api.post("/quit")
async def quit_server():
    log.info("Quit command received. Shutting down.")
    os.kill(os.getpid(), signal.SIGTERM)
    return {"quit": True}


# todo: set_*

class Server:

    def __init__(self, version: str, is_systemd: bool, log_level: int = 20) -> None:
        self.is_systemd: bool = is_systemd
        api.version = version
        api.debug = log_level < 20

    def startup(self) -> None:
        if self.is_systemd:
            log.info("Running in Systemd")
            uvicorn.run("server:api", fd=SYSTEMD_SOCKET_FD, workers=1)
        log.info("Liqctld server running...")
        uvicorn.run("server:api", host="127.0.0.1", port=DEFAULT_PORT, workers=1)

    @staticmethod
    @api.on_event("shutdown")
    def shutdown() -> None:
        log.info("Liqctld server shutting down")
        device_service.shutdown()
