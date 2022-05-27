#  Coolero - monitor and control your cooling and other devices
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

import getpass
import logging
from multiprocessing.connection import Client
from pathlib import Path

from coolero.settings import Settings

_LOG = logging.getLogger(__name__)
_SOCKET_NAME: str = 'coolerod.sock'
_DEFAULT_RESPONSE_WAIT_TIME: float = 1.0


class HwmonDaemonClient:
    """
    This class is used to speak the Daemon running in the background
    """
    _client_version: str = 'v1'

    def __init__(self, is_session_daemon: bool) -> None:
        self.is_session_daemon: bool = is_session_daemon
        if self.is_session_daemon:
            self._auth: bytes = getpass.getuser().encode('utf-8')
            self._socket: str = str(Settings.tmp_path.joinpath(_SOCKET_NAME))
        else:
            self._auth = Settings.app_path.joinpath(
                bytearray.fromhex('7265736f75726365732f69642e646174').decode('utf-8')
            ).read_bytes()
            self._socket = str(Settings.system_run_path.joinpath(_SOCKET_NAME))

        self._conn = Client(address=self._socket, family='AF_UNIX', authkey=self._auth)
        self.greet_daemon()

    def greet_daemon(self) -> None:
        self._conn.send(self._client_version)
        if self._conn.poll(_DEFAULT_RESPONSE_WAIT_TIME):
            response = self._conn.recv()
            if response != 'version supported':
                _LOG.error('Client version not supported by daemon: %s', response)
                self.close_connection()
                raise ValueError('Client version not supported by daemon')
            _LOG.info('Client version supported by daemon and greeting exchanged successfully')
            return
        raise ValueError('No greeting response from daemon')

    def apply_setting(self, path: Path, value: str) -> bool:
        self._conn.send([str(path), value])
        if self._conn.poll(_DEFAULT_RESPONSE_WAIT_TIME):
            response = self._conn.recv()
            if response == 'setting success':
                return True
        return False

    def close_connection(self) -> None:
        """This will close the connection to the daemon"""
        self._conn.send('close connection')
        if self._conn.poll(_DEFAULT_RESPONSE_WAIT_TIME):
            response = self._conn.recv()
            if response == 'bye':
                _LOG.info('Daemon connection closed')
                self._conn.close()
                return
        _LOG.warning('Error trying to close the Daemon connection')
        self._conn.close()

    def shutdown(self) -> None:
        """This will shut the daemon down"""
        self._conn.send('shutdown')
        if self._conn.poll(_DEFAULT_RESPONSE_WAIT_TIME):
            response = self._conn.recv()
            if response == 'bye':
                _LOG.info('Daemon shutdown')
                self._conn.close()
                return
        _LOG.warning('Error trying to shut the Daemon down')
        self._conn.close()
