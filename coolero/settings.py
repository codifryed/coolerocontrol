#  Coolero - monitor and control your cooling and other devices
#  Copyright (c) 2021  Guy Boldon
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

import json
import logging
import os
from collections import defaultdict
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Dict, Tuple, List, Optional

from PySide6 import QtCore
from PySide6.QtCore import QSettings

from models.speed_profile import SpeedProfile

_LOG = logging.getLogger(__name__)
IS_APP_IMAGE = os.environ.get("APPDIR") is not None
IS_FLATPAK = os.environ.get("FLATPAK_ID") is not None


@dataclass
class ProfileSetting:
    speed_profile: SpeedProfile
    fixed_duty: Optional[int] = None
    profile_temps: List[int] = field(default_factory=list)
    profile_duties: List[int] = field(default_factory=list)


@dataclass
class TempSourceSettings:
    profiles: Dict[str, List[ProfileSetting]] = field(default_factory=lambda: defaultdict(list))
    chosen_profile: Dict[str, ProfileSetting] = field(default_factory=dict)
    last_profile: Optional[Tuple[str, ProfileSetting]] = None


@dataclass
class ChannelSettings:
    channels: Dict[str, TempSourceSettings] = field(default_factory=lambda: defaultdict(TempSourceSettings))


@dataclass(frozen=True, order=True)
class DeviceSetting:
    name: str
    id: int


@dataclass
class SavedProfiles:
    profiles: Dict[DeviceSetting, ChannelSettings] = field(default_factory=lambda: defaultdict(ChannelSettings))


def serialize(path: Path, settings: Dict) -> None:
    with open(path, "w", encoding='utf-8') as write:
        json.dump(settings, write, indent=2)


def deserialize(path: Path) -> Dict:
    with open(path, "r", encoding='utf-8') as reader:
        return dict(json.loads(reader.read()))


class UserSettings(str, Enum):
    SAVE_WINDOW_SIZE = 'save_window_size'
    WINDOW_SIZE = 'window_size'
    WINDOW_POSITION = 'window_position'
    ENABLE_LIGHT_THEME = 'enable_light_theme'
    HIDE_ON_CLOSE = 'hide_on_close'
    UI_SCALE_FACTOR = 'ui_scale_factor'
    CONFIRM_EXIT = 'confirm_exit'
    ENABLE_SMOOTHING = 'enable_smoothing'
    CHECK_FOR_UPDATES = 'check_for_updates'
    PROFILES = 'profiles/v1'
    APPLIED_PROFILES = 'applied_profiles/v1'

    def __str__(self) -> str:
        return str.__str__(self)


class Settings:
    """This class provides static Settings access to all files in the application"""
    application_path: Path = Path(__file__).resolve().parent
    user: QSettings = QtCore.QSettings('coolero', 'Coolero')
    app: Dict = {}
    theme: Dict = {}
    _saved_profiles: SavedProfiles = user.value(UserSettings.PROFILES, defaultValue=SavedProfiles())  # type: ignore
    _last_applied_profiles: SavedProfiles = user.value(
        UserSettings.APPLIED_PROFILES, defaultValue=SavedProfiles())  # type: ignore

    _app_json_path = application_path.joinpath('resources/settings.json')
    if not _app_json_path.is_file():
        _LOG.fatal('FATAL: "settings.json" not found! check in the folder %s', _app_json_path)
    app = deserialize(_app_json_path)

    user_theme: str = "default"
    is_light_theme = user.value(UserSettings.ENABLE_LIGHT_THEME, defaultValue=False, type=bool)
    if is_light_theme:
        user_theme = "bright_theme"
    _theme_json_path = application_path.joinpath(f'resources/themes/{user_theme}.json')
    if not _theme_json_path.is_file():
        _LOG.warning('"gui/themes/%s.json" not found! check in the folder %s', user_theme, _theme_json_path)
    theme = deserialize(_theme_json_path)

    @staticmethod
    def save_profiles() -> None:
        _LOG.debug('Saving Profiles: %s', Settings._saved_profiles)
        Settings.user.setValue(UserSettings.PROFILES, Settings._saved_profiles)

    @staticmethod
    def save_last_applied_profiles() -> None:
        _LOG.debug('Saving Last Applied Profiles: %s', Settings._last_applied_profiles)
        Settings.user.setValue(UserSettings.APPLIED_PROFILES, Settings._last_applied_profiles)

    @staticmethod
    def get_temp_source_settings(
            device_name: str, device_id: int, channel_name: str
    ) -> TempSourceSettings:
        saved_profiles = Settings._saved_profiles.profiles
        device_setting = DeviceSetting(device_name, device_id)
        return saved_profiles[device_setting].channels[channel_name]

    @staticmethod
    def get_temp_source_chosen_profile(
            device_name: str, device_id: int, channel_name: str, temp_source_name: str
    ) -> Optional[ProfileSetting]:
        return Settings.get_temp_source_settings(
            device_name, device_id, channel_name
        ).chosen_profile.get(temp_source_name)

    @staticmethod
    def get_temp_source_profiles(
            device_name: str, device_id: int, channel_name: str, temp_source_name: str
    ) -> List[ProfileSetting]:
        return Settings.get_temp_source_settings(
            device_name, device_id, channel_name
        ).profiles[temp_source_name]

    @staticmethod
    def save_chosen_profile_for_temp_source(
            device_name: str, device_id: int, channel_name: str, temp_source_name: str, speed_profile: SpeedProfile
    ) -> None:
        temp_source_settings = Settings.get_temp_source_settings(device_name, device_id, channel_name)
        temp_source_settings.chosen_profile[temp_source_name] = ProfileSetting(speed_profile)
        Settings.save_profiles()

    @staticmethod
    def save_fixed_profile(
            device_name: str, device_id: int, channel_name: str, temp_source_name: str, fixed_duty: int
    ) -> None:
        temp_source_settings = Settings.get_temp_source_settings(device_name, device_id, channel_name)
        temp_source_settings.chosen_profile[temp_source_name] = ProfileSetting(
            SpeedProfile.FIXED, fixed_duty=fixed_duty)
        for profile in temp_source_settings.profiles[temp_source_name]:
            if profile.speed_profile == SpeedProfile.FIXED:
                profile.fixed_duty = fixed_duty
                break
        else:
            temp_source_settings.profiles[temp_source_name].append(
                ProfileSetting(SpeedProfile.FIXED, fixed_duty=fixed_duty)
            )
        Settings.save_profiles()

    @staticmethod
    def save_custom_profile(
            device_name: str, device_id: int, channel_name: str, temp_source_name: str,
            temps: List[int], duties: List[int]
    ) -> None:
        temp_source_settings = Settings.get_temp_source_settings(device_name, device_id, channel_name)
        temp_source_settings.chosen_profile[temp_source_name] = ProfileSetting(
            SpeedProfile.CUSTOM, profile_temps=temps, profile_duties=duties)
        for profile in temp_source_settings.profiles[temp_source_name]:
            if profile.speed_profile == SpeedProfile.CUSTOM:
                profile.profile_temps = temps
                profile.profile_duties = duties
                break
        else:
            temp_source_settings.profiles[temp_source_name].append(
                ProfileSetting(SpeedProfile.CUSTOM, profile_temps=temps, profile_duties=duties)
            )
        Settings.save_profiles()

    @staticmethod
    def get_last_applied_temp_source_settings(
            device_name: str, device_id: int, channel_name: str
    ) -> TempSourceSettings:
        last_applied_profiles = Settings._last_applied_profiles.profiles
        device_setting = DeviceSetting(device_name, device_id)
        return last_applied_profiles[device_setting].channels[channel_name]

    @staticmethod
    def get_last_applied_profile_for_channel(
            device_name: str, device_id: int, channel_name: str
    ) -> Optional[Tuple[str, ProfileSetting]]:
        return Settings.get_last_applied_temp_source_settings(device_name, device_id, channel_name).last_profile

    @staticmethod
    def save_applied_fixed_profile(
            device_name: str, device_id: int, channel_name: str, temp_source_name: str, applied_fixed_duty: int
    ) -> None:
        last_applied_temp_source_settings = Settings.get_last_applied_temp_source_settings(
            device_name, device_id, channel_name)
        last_applied_temp_source_settings.last_profile = (
            temp_source_name, ProfileSetting(SpeedProfile.FIXED, fixed_duty=applied_fixed_duty)
        )
        Settings.save_last_applied_profiles()

    @staticmethod
    def save_applied_custom_profile(
            device_name: str, device_id: int, channel_name: str, temp_source_name: str,
            temps: List[int], duties: List[int]
    ) -> None:
        last_applied_temp_source_settings = Settings.get_last_applied_temp_source_settings(
            device_name, device_id, channel_name)
        last_applied_temp_source_settings.last_profile = (
            temp_source_name, ProfileSetting(SpeedProfile.CUSTOM, profile_temps=temps, profile_duties=duties)
        )
        Settings.save_last_applied_profiles()

    @staticmethod
    def clear_applied_profile_for_channel(
            device_name: str, device_id: int, channel_name: str
    ) -> None:
        Settings.get_last_applied_temp_source_settings(
            device_name, device_id, channel_name
        ).last_profile = None
        Settings.save_last_applied_profiles()

    @staticmethod
    def save_app_settings() -> None:
        """
        This is just a helper function for doing things like updating the version per script.
        This should not be called during the normal run of the application
        """
        if not Settings.app:
            return
        serialize(Settings._app_json_path, Settings.app)


class FeatureToggle:
    lighting: bool = False
    testing: bool = False
