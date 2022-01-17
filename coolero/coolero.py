#!/usr/bin/env python3

#  Coolero - monitor and control your cooling and other devices Copyright (c) 2021  Guy Boldon
#  All credit for basis of the user interface (GUI) goes to: Wanderson M.Pimenta
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

# nuitka-project: --standalone
# nuitka-project: --follow-imports
# nuitka-project: --include-data-dir=./coolero/config=config
# nuitka-project: --include-data-dir=./coolero/resources=resources
# nuitka-project: --plugin-enable=anti-bloat,pyside6,pylint-warnings,numpy
# nuitka-project: --include-module=services.liquidctl_device_extractors
# nuitka-project: --static-libpython=no
# nuitka-project: --lto=yes
# nuitka-project: --prefer-source-code
# nuitka-project: --python-flag=-S,-O,no_docstrings
# nuitka-project: --linux-onefile-icon=metadata/org.coolero.Coolero.png

import argparse
import logging.config
import os
import sys
import tempfile
from logging.handlers import RotatingFileHandler
from pathlib import Path
from typing import Optional

import setproctitle
from PySide6 import QtCore
from PySide6.QtCore import QTimer, QCoreApplication, QEvent, QSize, QPoint
from PySide6.QtGui import QColor, Qt, QIcon, QAction
from PySide6.QtWidgets import QMainWindow, QGraphicsDropShadowEffect, QApplication, QSystemTrayIcon, QMenu, QMessageBox

from dialogs.quit_dialog import QuitDialog
from dialogs.udev_rules_dialog import UDevRulesDialog
from exceptions.device_communication_error import DeviceCommunicationError
from services.app_updater import AppUpdater
from services.dynamic_buttons import DynamicButtons
from settings import Settings, UserSettings, IS_APP_IMAGE
from view.core.functions import Functions
from view.uis.pages.info_page import InfoPage
from view.uis.pages.settings_page import SettingsPage
from view.uis.windows.main_window import SetupMainWindow, UI_MainWindow, MainFunctions
from view.uis.windows.splash_screen.splash_screen_style import SPLASH_SCREEN_STYLE
from view.uis.windows.splash_screen.ui_splash_screen import Ui_SplashScreen
from view_models.devices_view_model import DevicesViewModel

logging.config.fileConfig(Settings.application_path.joinpath('config/logging.conf'), disable_existing_loggers=False)
_LOG = logging.getLogger(__name__)


class Initialize(QMainWindow):

    def __init__(self) -> None:
        super().__init__()
        _LOG.info("Coolero is initializing...")
        self._load_progress_counter: int = 0

        self.app_settings = Settings.app
        self.user_settings = Settings.user
        self.user_settings.setValue('version', self.app_settings['version'])
        QApplication.setApplicationName(self.app_settings['app_name'])
        QApplication.setApplicationVersion(self.app_settings['version'])
        self.theme = Settings.theme

        parser = argparse.ArgumentParser(
            description='monitor and control your cooling and other devices',
            exit_on_error=False
        )
        parser.add_argument('-v', '--version', action='version',
                            version=f'{self.app_settings["app_name"]} v{self.app_settings["version"]}')
        parser.add_argument('--debug', action='store_true', help='turn on debug logging')
        args = parser.parse_args()
        if args.debug:
            log_path = Path(tempfile.gettempdir() + '/coolero/')
            log_path.mkdir(mode=0o700, exist_ok=True)
            log_filename = log_path.joinpath('coolero.log')
            file_handler = RotatingFileHandler(
                filename=log_filename, maxBytes=10485760, backupCount=5, encoding='utf-8'
            )
            log_formatter = logging.getLogger('root').handlers[0].formatter
            file_handler.setFormatter(log_formatter)
            logging.getLogger('root').setLevel(logging.DEBUG)
            logging.getLogger('root').addHandler(file_handler)
            logging.getLogger('matplotlib').setLevel(logging.INFO)
            logging.getLogger('matplotlib').addHandler(file_handler)
            logging.getLogger('apscheduler').setLevel(logging.INFO)
            logging.getLogger('apscheduler').addHandler(file_handler)
            logging.getLogger('liquidctl').setLevel(logging.DEBUG)
            logging.getLogger('liquidctl').addHandler(file_handler)
            _LOG.debug('DEBUG level enabled')

        # Setup splash window
        self.ui = Ui_SplashScreen()
        self.ui.setupUi(self)
        splash_style = SPLASH_SCREEN_STYLE.format(
            _bg_color=self.theme["app_color"]["bg_one"],
            _title_color=self.theme["app_color"]["text_title"],
            _color=self.theme["app_color"]["text_foreground"],
            _progress_bg_color=self.theme["app_color"]["bg_two"],
            _progress_color=self.theme["app_color"]["white"],
            _progress_from_color=self.theme["app_color"]["icon_hover"],
            _progress_to_color=self.theme["app_color"]["context_pressed"]
        )
        self.ui.dropShadowFrame.setStyleSheet(splash_style)
        self.ui.label_title.setStyleSheet(splash_style)
        self.ui.label_description.setStyleSheet(splash_style)
        self.ui.label_loading.setStyleSheet(splash_style)
        self.ui.label_version.setStyleSheet(splash_style)
        self.ui.progressBar.setStyleSheet(splash_style)
        self.setWindowFlag(Qt.FramelessWindowHint)
        self.setAttribute(Qt.WA_TranslucentBackground)
        self.shadow = QGraphicsDropShadowEffect(self)
        self.shadow.setBlurRadius(20)
        self.shadow.setXOffset(0)
        self.shadow.setYOffset(0)
        self.shadow.setColor(QColor(0, 0, 0, 60))
        self.ui.dropShadowFrame.setGraphicsEffect(self.shadow)

        self.ui.label_loading.setText("<strong>Initializing</strong>")
        self.ui.label_version.setText("<strong>version</strong>: " + self.app_settings["version"])

        self.timer = QTimer()
        self.timer.timeout.connect(self.init_devices)
        self.timer.start(10)

        self.main = MainWindow()
        self.main.devices_view_model = DevicesViewModel()
        # from services.dynamic_buttons import DynamicButtons
        self.main.dynamic_buttons = DynamicButtons(
            self.main.devices_view_model,
            self.main
        )

        self.show()

    def init_devices(self) -> None:
        try:
            should_check_for_update: bool = self.user_settings.value(
                UserSettings.CHECK_FOR_UPDATES, defaultValue=False, type=bool
            ) and IS_APP_IMAGE
            if self._load_progress_counter == 0:
                self.main.devices_view_model.schedule_status_updates()

                if should_check_for_update:
                    self.ui.label_loading.setText("<strong>Checking</strong> for updates")
            elif self._load_progress_counter == 10:
                if should_check_for_update:
                    AppUpdater.run(self)

                self.ui.label_loading.setText("<strong>Initializing</strong> CPU connection")
            elif self._load_progress_counter == 20:
                self.main.devices_view_model.init_cpu_repo()

                self.ui.label_loading.setText("<strong>Initializing</strong> GPU Connection")
            elif self._load_progress_counter == 35:
                self.main.devices_view_model.init_gpu_repo()

                self.ui.label_loading.setText("<strong>Initializing</strong> Liquidctl devices")
            elif self._load_progress_counter == 60:
                try:
                    self.main.devices_view_model.init_liquidctl_repo()
                except DeviceCommunicationError as ex:
                    _LOG.error('Liquidctl device communication error: %s', ex)
                    UDevRulesDialog(self).run()

                self.ui.label_loading.setText("<strong>Initializing</strong> the UI")
            elif self._load_progress_counter == 90:
                # wire up core logic:
                self.main.devices_view_model.subscribe(self.main.ui.system_overview_canvas)
                self.main.dynamic_buttons.create_menu_buttons_from_liquidctl_devices()
                self.main.ui.left_column.menus.info_page_layout.setAlignment(Qt.AlignTop)
                self.main.ui.left_column.menus.info_page_layout.addWidget(
                    InfoPage(self.main.devices_view_model.devices)
                )
                self.main.ui.left_column.menus.settings_page_layout.addWidget(SettingsPage())

            elif self._load_progress_counter >= 100:
                self.timer.stop()
                _LOG.info("Displaying Main UI Window...")
                self.main.show()
                self.close()

            self._load_progress_counter += 1
            self.ui.progressBar.setValue(self._load_progress_counter)
        except BaseException as ex:
            _LOG.fatal('Unexpected Error', exc_info=ex)
            _LOG.info("Shutting down...")
            self.main.devices_view_model.shutdown()
            self.close()


class MainWindow(QMainWindow):

    def __init__(self) -> None:
        super().__init__()

        self.ui = UI_MainWindow()
        self.ui.setup_ui(self)
        self.dragPos = None
        self.active_left_sub_menu: str = ''
        self.devices_view_model: DevicesViewModel = None
        self.dynamic_buttons: DynamicButtons = None

        self.app_settings = Settings.app
        self.user_settings = Settings.user

        self.hide_grips = True  # Show/Hide resize grips
        SetupMainWindow.setup_gui(self)

        # restore window size & position
        if self.user_settings.contains(UserSettings.WINDOW_SIZE):
            try:
                self.resize(
                    self.user_settings.value(
                        UserSettings.WINDOW_SIZE,
                        defaultValue=QSize(self.app_settings["startup_size"][0], self.app_settings["startup_size"][1]),
                        type=QSize
                    )
                )
                self.move(
                    self.user_settings.value(
                        UserSettings.WINDOW_POSITION,
                        defaultValue=QPoint(200, 200),
                        type=QPoint
                    )
                )
                _LOG.debug('Loaded saved window size')
            except BaseException as ex:
                _LOG.error('Unable to get and restore saved window geometry: %s', ex)

        self.tray_menu = QMenu(self)
        self.tray_menu.addAction(
            QAction(
                self.app_settings['app_name'], self, icon=QIcon(icon), triggered=None, enabled=False
            )  # type: ignore[call-overload]
        )
        self.tray_menu.addSeparator()
        self.tray_menu.addAction(  # shortcut='Ctrl+h' - shortcuts don't appear to work for the sys tray actions?
            QAction('&Show Coolero', self, triggered=self.show_main_window))  # type: ignore[call-overload]
        self.tray_menu.addSeparator()
        self.tray_menu.addAction(
            QAction('&Quit Coolero', self, triggered=self.force_close))  # type: ignore[call-overload]
        self.tray = QSystemTrayIcon(self)
        self.tray.setIcon(icon)
        self.tray.setVisible(True)
        self.tray.setContextMenu(self.tray_menu)

    def show_main_window(self) -> None:
        if not self.isVisible():
            self.setVisible(True)
        self.activateWindow()

    # main menu btn
    def btn_clicked(self) -> None:

        btn = SetupMainWindow.setup_btns(self)
        btn_id = btn.objectName()
        _LOG.debug('Button %s, clicked!', btn_id)

        # home btn
        if btn_id == "btn_system":
            self.ui.left_menu.select_only_one(btn.objectName())
            self.clear_left_sub_menu()
            MainFunctions.set_page(self, self.ui.load_pages.system_overview)

        # Info and Settings combined:
        elif btn_id in ["btn_settings", "btn_info"]:
            if not MainFunctions.left_column_is_visible(self):
                self.dynamic_buttons.uncheck_all_channel_buttons()
                MainFunctions.toggle_left_column(self)
                self.ui.left_menu.select_only_one_tab(btn_id)
                self.active_left_sub_menu = btn_id
            elif btn_id == self.active_left_sub_menu:
                # close side menu
                self.ui.left_menu.deselect_all_tab()
                self.active_left_sub_menu = ''
                MainFunctions.toggle_left_column(self)
            else:
                self.active_left_sub_menu = btn.objectName()
                self.ui.left_menu.select_only_one_tab(btn.objectName())

            if btn_id == "btn_settings":
                MainFunctions.set_left_column_menu(
                    self,
                    menu=self.ui.left_column.menus.settings_page,
                    title="Settings",
                    icon_path=Functions.set_svg_icon("icon_settings.svg")
                )
            elif btn_id == "btn_info":
                MainFunctions.set_left_column_menu(
                    self,
                    menu=self.ui.left_column.menus.info_page,
                    title="Info",
                    icon_path=Functions.set_svg_icon("icon_info.svg")
                )
        else:
            self.dynamic_buttons.set_liquidctl_device_page(btn_id)

    def clear_left_sub_menu(self) -> None:
        self.ui.left_menu.deselect_all_tab()
        if MainFunctions.left_column_is_visible(self):
            MainFunctions.toggle_left_column(self)

    def btn_released(self) -> None:
        btn = SetupMainWindow.setup_btns(self)
        _LOG.debug('Button %s, released!', btn.objectName())

    def resizeEvent(self, event: QEvent) -> None:
        SetupMainWindow.resize_grips(self)
        if self.ui.device_column_frame.width() > 0:
            self.ui.device_column_frame.setMinimumWidth(int((self.width() - self.ui.left_menu_frame.width()) / 2))

    def mousePressEvent(self, event: QEvent) -> None:
        self.dragPos = event.globalPosition().toPoint()

    def closeEvent(self, event: QEvent) -> None:
        """Shutdown or minimize to tray"""
        if self.user_settings.value(UserSettings.HIDE_ON_CLOSE, defaultValue=False, type=bool):
            self.hide()
            event.ignore()
        else:
            self.shutdown(event)

    def force_close(self) -> None:
        self.shutdown()

    def shutdown(self, event: Optional[QEvent] = None) -> None:
        """Shutdown process"""
        reply = QuitDialog(self).run() \
            if Settings.user.value(UserSettings.CONFIRM_EXIT, defaultValue=True, type=bool) else QMessageBox.Yes
        if reply == QMessageBox.Yes:
            _LOG.info("Shutting down...")
            self.devices_view_model.shutdown()
            if self.user_settings.value(UserSettings.SAVE_WINDOW_SIZE, defaultValue=False, type=bool):
                self.user_settings.setValue(UserSettings.WINDOW_SIZE, self.size())
                self.user_settings.setValue(UserSettings.WINDOW_POSITION, self.pos())
                _LOG.debug('Saved window size in user settings')
            else:
                self.user_settings.remove(UserSettings.WINDOW_SIZE)
                self.user_settings.remove(UserSettings.WINDOW_POSITION)
            if event is not None:
                super(MainWindow, self).closeEvent(event)
            else:
                self.close()
                app.quit()
        elif event is not None:
            event.ignore()


if __name__ == "__main__":
    setproctitle.setproctitle("coolero")
    QCoreApplication.setAttribute(QtCore.Qt.AA_ShareOpenGLContexts)
    QApplication.setAttribute(QtCore.Qt.AA_EnableHighDpiScaling)
    QApplication.setAttribute(QtCore.Qt.AA_UseHighDpiPixmaps)
    QApplication.setAttribute(Qt.AA_UseDesktopOpenGL)
    QApplication.setAttribute(Qt.AA_Use96Dpi)
    os.environ["QT_FONT_DPI"] = "96"  # this appears to need to be set to keep things sane
    os.environ["QT_SCALE_FACTOR"] = str(  # scale performs better than higher dpi
        Settings.user.value(UserSettings.UI_SCALE_FACTOR, defaultValue=1.0, type=float)
    )
    app = QApplication(sys.argv)
    icon = QIcon(str(Settings.application_path.joinpath('resources/images/icon.ico')))
    app.setWindowIcon(icon)
    window = Initialize()
    sys.exit(app.exec())
