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

import logging
from collections import defaultdict
from typing import Dict

from apscheduler.executors.pool import ThreadPoolExecutor
from apscheduler.schedulers.background import BackgroundScheduler
from apscheduler.triggers.date import DateTrigger
from jeepney import DBusAddress, new_method_call, Message
from jeepney.io.blocking import open_dbus_connection, DBusConnection

from coolero.settings import Settings, IS_FLATPAK
from coolero.view.core.functions import Functions

_LOG = logging.getLogger(__name__)


class Notifications:
    _scheduler: BackgroundScheduler = BackgroundScheduler(
        executors={'default': ThreadPoolExecutor(1)},
        job_defaults={'misfire_grace_time': 3, 'coalesce': False, 'replace_existing': False, 'max_instances': 10}
    )
    _dbus_address: DBusAddress = DBusAddress('/org/freedesktop/Notifications',
                                             bus_name='org.freedesktop.Notifications',
                                             interface='org.freedesktop.Notifications')
    _dbus_method: str = 'Notify'
    _dbus_message_body_signature: str = 'susssasa{sv}i'
    _app_name: str = 'org.coolero.Coolero'
    _title: str = Settings.app['app_name']
    _id: str = 'desktop_notification'
    _timeout: int = -1  # -1 = default

    def __init__(self) -> None:
        self._scheduler.start()
        self._previous_message_ids: Dict[str, int] = defaultdict(lambda: 0)
        if IS_FLATPAK:
            self._icon: str = self._app_name
        else:
            self._icon = Functions.set_image('logo_200.png')
        try:
            self._connection: DBusConnection = open_dbus_connection(bus='SESSION')
        except BaseException as ex:
            _LOG.error('Could not open DBus connection for notifications', exc_info=ex)

    def shutdown(self) -> None:
        self._scheduler.shutdown()
        if self._connection is not None:
            self._connection.close()

    def settings_applied(self, device_name: str = '') -> None:
        """This will take the response of the applied-settings-function and send a notification of completion"""
        if self._connection is None or device_name is None:
            return
        msg: str = 'Settings applied'
        if device_name:
            msg += f' to\n{device_name}'
        self._scheduler.add_job(
            lambda: self._send_message(msg, device_name),
            DateTrigger(),  # defaults to now()
            id=self._id
        )

    def _send_message(self, msg: str, device_name: str) -> None:
        try:
            dbus_msg: Message = new_method_call(
                self._dbus_address,
                self._dbus_method,
                self._dbus_message_body_signature,
                (
                    self._app_name,
                    self._previous_message_ids[device_name],  # replacing any previous notification
                    self._icon,
                    self._title,
                    msg,
                    [], {},  # Actions, hints
                    self._timeout,  # expire_timeout (-1 = default)
                )
            )
            reply: Message = self._connection.send_and_get_reply(dbus_msg)
            if reply.body is not None:
                self._previous_message_ids[device_name] = reply.body[0]
                _LOG.debug('DBus Notification received with ID: %s', reply.body[0])
            else:
                _LOG.warning('DBus Notification response body was empty')
        except BaseException as ex:
            _LOG.error('DBus messaging error', exc_info=ex)
