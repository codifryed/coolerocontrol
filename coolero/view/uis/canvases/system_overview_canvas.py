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

import logging
from datetime import datetime
from typing import Optional, List, Dict

from matplotlib.animation import Animation, FuncAnimation
from matplotlib.artist import Artist
from matplotlib.backend_bases import PickEvent
from matplotlib.backends.backend_qt5agg import FigureCanvasQTAgg
from matplotlib.figure import Figure
from matplotlib.legend import Legend
from matplotlib.lines import Line2D

from models.device import Device, DeviceType
from models.status import Status
from settings import Settings
from view_models.device_observer import DeviceObserver
from view_models.device_subject import DeviceSubject

_LOG = logging.getLogger(__name__)
CPU_TEMP: str = 'CPU Temp'
CPU_LOAD: str = 'CPU Load'
GPU_LOAD: str = 'GPU Load'
GPU_TEMP: str = 'GPU Temp'
DEVICE_TEMP: str = ' Device Temp'
DEVICE_LIQUID_TEMP: str = ' Liquid Temp'
DEVICE_PUMP: str = ' Pump Duty'
DEVICE_FAN: str = ' Fan Duty'
DRAW_INTERVAL_MS: int = 1000


class SystemOverviewCanvas(FigureCanvasQTAgg, FuncAnimation, DeviceObserver):
    """Class to plot and animate System Overview histogram"""

    _cpu_lines_initialized: bool = False
    _gpu_lines_initialized: bool = False
    _liquidctl_lines_initialized: bool = False

    def __init__(self,
                 width: int = 16,  # width/height ratio & inches for print
                 height: int = 9,
                 dpi: int = 120,
                 bg_color: str = Settings.theme['app_color']['bg_two'],
                 text_color: str = Settings.theme['app_color']['text_foreground'],
                 title_color: str = Settings.theme["app_color"]["text_title"]
                 ) -> None:
        self._bg_color = bg_color
        self._text_color = text_color
        self._devices: List[Device] = []
        # todo: create button for 5, 10 and 15 size charts (quasi zoom)
        self.x_limit: int = 5 * 60  # the age, in seconds, of data to display

        # Setup
        self.fig = Figure(figsize=(width, height), dpi=dpi, layout='tight', facecolor=bg_color, edgecolor=text_color)
        self.axes = self.fig.add_subplot(111, facecolor=bg_color)
        self.legend: Legend
        if Settings.app["custom_title_bar"]:
            self.axes.set_title('System Overview', color=title_color, size='large')
        self.axes.set_ylim(0, 101)
        self.axes.set_xlim(self.x_limit, 0)  # could make this modifiable to scaling & zoom

        # Grid
        self.axes.grid(True, linestyle='dotted', color=text_color, alpha=0.5)
        self.axes.margins(x=0, y=0.05)
        self.axes.tick_params(colors=text_color)
        # todo: dynamically set by button above
        self.axes.set_xticks([30, 60, 120, 180, 240, 300], ['30s', '1m', '2m', '3m', '4m', '5m'])
        # self.axes.set_yticks([10, 20, 30, 40, 50, 60, 70, 80, 90, 100])
        self.axes.set_yticks(
            [10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
            ['10°/%', '20°/%', '30°/%', '40°/%', '50°/%', '60°/%', '70°/%', '80°/%', '90°/%', '100°/%', ])
        self.axes.spines['top'].set_edgecolor(text_color + '00')
        self.axes.spines['right'].set_edgecolor(text_color + '00')
        self.axes.spines[['bottom', 'left']].set_edgecolor(text_color)

        # Lines
        self.lines: List[Line2D] = []
        self.legend_artists: Dict[Artist, Line2D] = {}

        # Interactions
        self.fig.canvas.mpl_connect('pick_event', self._on_pick)

        # Initialize
        FigureCanvasQTAgg.__init__(self, self.fig)
        FuncAnimation.__init__(self, self.fig, func=self.draw_frame, interval=DRAW_INTERVAL_MS, blit=True)

    def draw_frame(self, frame: int) -> List[Artist]:
        """Is used to draw every frame of the chart animation"""
        now: datetime = datetime.now()
        self._set_cpu_data(now)
        self._set_gpu_data(now)
        self._set_lc_device_data(now)
        if frame > 0 and frame % 8 == 0:  # clear the blit cache of strange artifacts every so often
            self._redraw_whole_canvas()
        return self.lines

    def notify_me(self, subject: DeviceSubject) -> None:
        if self._devices:
            return
        self._devices = subject.devices
        cpu = self._get_first_device_with_type(DeviceType.CPU)
        if cpu is not None:
            self._initialize_cpu_lines(cpu)
        gpu = self._get_first_device_with_type(DeviceType.GPU)
        if gpu is not None:
            self._initialize_gpu_lines(gpu)
        devices = self._get_devices_with_type(DeviceType.LIQUIDCTL)
        if devices:
            self._initialize_liquidctl_lines(devices)
        self._redraw_whole_canvas()

    def init_legend(self, bg_color: str, text_color: str) -> Legend:
        legend = self.axes.legend(loc='upper left', facecolor=bg_color, edgecolor=text_color)
        for legend_line, legend_text, ax_line in zip(legend.get_lines(), legend.get_texts(), self.lines):
            legend_line.set_picker(True)
            legend_text.set_color(text_color)
            legend_text.set_picker(True)
            self.legend_artists[legend_line] = ax_line
            self.legend_artists[legend_text] = ax_line
        return legend

    def _set_cpu_data(self, now: datetime) -> None:
        cpu = self._get_first_device_with_type(DeviceType.CPU)
        if self._cpu_lines_initialized and cpu:
            cpu_history: List[Status] = cpu.status_history
            cpu_temps: List[float] = []
            cpu_loads: List[float] = []
            cpu_status_ages: List[int] = []
            for status in cpu_history[-self.x_limit:]:
                cpu_temps.append(status.device_temperature)
                cpu_loads.append(status.load_percent)
                cpu_status_ages.append(
                    (now - status.timestamp).seconds
                )

            self._get_line_by_label(CPU_TEMP).set_data(cpu_status_ages, cpu_temps)
            self._get_line_by_label(CPU_LOAD).set_data(cpu_status_ages, cpu_loads)

    def _set_gpu_data(self, now: datetime) -> None:
        gpu = self._get_first_device_with_type(DeviceType.GPU)
        if self._gpu_lines_initialized and gpu:
            gpu_history: List[Status] = gpu.status_history
            gpu_temps: List[float] = []
            gpu_loads: List[float] = []
            gpu_status_ages: List[int] = []
            for status in gpu_history[-self.x_limit:]:
                gpu_temps.append(status.device_temperature)
                gpu_loads.append(status.load_percent)
                gpu_status_ages.append(
                    (now - status.timestamp).seconds
                )
            self._get_line_by_label(GPU_TEMP).set_data(gpu_status_ages, gpu_temps)
            self._get_line_by_label(GPU_LOAD).set_data(gpu_status_ages, gpu_loads)

    def _set_lc_device_data(self, now: datetime) -> None:
        if not self._liquidctl_lines_initialized:
            return
        for device in self._get_devices_with_type(DeviceType.LIQUIDCTL):
            device_temps: List[float] = []
            device_liquid_temps: List[float] = []
            device_pump: List[float] = []
            device_fan: List[float] = []
            device_status_ages: List[int] = []
            for status in device.status_history[-self.x_limit:]:
                if status.device_temperature:
                    device_temps.append(status.device_temperature)
                if status.liquid_temperature:
                    device_liquid_temps.append(status.liquid_temperature)
                if status.pump_duty:
                    device_pump.append(status.pump_duty)
                if status.fan_duty:
                    device_fan.append(status.fan_duty)
                device_status_ages.append(
                    (now - status.timestamp).seconds
                )
            if device_temps:
                self._get_line_by_label(
                    device.device_name_short + DEVICE_TEMP
                ).set_data(device_status_ages, device_temps)
            if device_liquid_temps:
                self._get_line_by_label(
                    device.device_name_short + DEVICE_LIQUID_TEMP
                ).set_data(device_status_ages, device_liquid_temps)
            if device_pump:
                self._get_line_by_label(
                    device.device_name_short + DEVICE_PUMP
                ).set_data(device_status_ages, device_pump)
            if device_fan:
                self._get_line_by_label(
                    device.device_name_short + DEVICE_FAN
                ).set_data(device_status_ages, device_fan)

    def _get_first_device_with_type(self, device_type: DeviceType) -> Optional[Device]:
        return next(
            iter(self._get_devices_with_type(device_type)),
            None
        )

    def _get_devices_with_type(self, device_type: DeviceType) -> List[Device]:
        return [device for device in self._devices if device.device_type == device_type]

    def _initialize_cpu_lines(self, cpu: Device) -> None:
        lines_cpu = [
            Line2D([], [], color=cpu.device_color, label=CPU_TEMP, linewidth=2),
            Line2D([], [], color=cpu.device_color, label=CPU_LOAD, linestyle='dashed', linewidth=1)
        ]
        self.lines.extend(lines_cpu)
        for line in lines_cpu:
            self.axes.add_line(line)
        self._cpu_lines_initialized = True
        _LOG.debug('initialized cpu lines')

    def _initialize_gpu_lines(self, gpu: Device) -> None:
        lines_gpu = [
            Line2D([], [], color=gpu.device_color, label=GPU_TEMP, linewidth=2),
            Line2D([], [], color=gpu.device_color, label=GPU_LOAD, linestyle='dashed', linewidth=1)
        ]
        self.lines.extend(lines_gpu)
        for line in lines_gpu:
            self.axes.add_line(line)
        self._gpu_lines_initialized = True
        _LOG.debug('initialized gpu lines')

    def _initialize_liquidctl_lines(self, devices: List[Device]) -> None:
        for device in devices:
            lines_liquidctl = []
            if device.status.device_temperature:
                lines_liquidctl.append(
                    # todo: device line colors based on cycle of colors.
                    Line2D([], [], color=device.device_color,
                           label=device.device_name_short + DEVICE_TEMP,
                           linewidth=2))
            if device.status.liquid_temperature:
                lines_liquidctl.append(
                    Line2D([], [], color=device.device_color,
                           label=device.device_name_short + DEVICE_LIQUID_TEMP,
                           linewidth=2))
            if device.status.pump_duty:
                lines_liquidctl.append(
                    Line2D([], [], color=device.device_color,
                           label=device.device_name_short + DEVICE_PUMP,
                           linestyle='dashed', linewidth=1))
            if device.status.fan_duty:
                lines_liquidctl.append(
                    Line2D([], [], color=device.device_color,
                           label=device.device_name_short + DEVICE_FAN,
                           linestyle='dashdot', linewidth=1))
            self.lines.extend(lines_liquidctl)
            for line in lines_liquidctl:
                self.axes.add_line(line)
        self._liquidctl_lines_initialized = True
        _LOG.debug('initialized liquidctl lines')

    def _redraw_whole_canvas(self) -> None:
        self.legend = self.init_legend(self._bg_color, self._text_color)
        self._blit_cache.clear()
        self._init_draw()
        self.draw()

    def _get_line_by_label(self, label: str) -> Line2D:
        return next(line for line in self.lines if line.get_label() == label)

    def _on_pick(self, event: PickEvent) -> None:
        chosen_artist = event.artist
        ax_line = self.legend_artists.get(chosen_artist)
        if ax_line is None:
            _LOG.error('Chosen artist in system overview legend was not found')
            return
        is_visible: bool = not ax_line.get_visible()
        ax_line.set_visible(is_visible)
        for artist in (artist for artist, line in self.legend_artists.items() if line == ax_line):
            artist.set_alpha(1.0 if is_visible else 0.2)
        self._blit_cache.clear()
        self._init_draw()
        self.draw()
        Animation._step(self)
