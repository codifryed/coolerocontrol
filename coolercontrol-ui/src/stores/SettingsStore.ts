/*
 * CoolerControl - monitor and control your cooling and other devices
 * Copyright (c) 2023  Guy Boldon
 * |
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * |
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * |
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

import {defineStore} from "pinia"
import {Function, FunctionsDTO, Profile, ProfilesDTO} from "@/models/Profile"
import type {Ref} from "vue"
import {reactive, ref, toRaw, watch} from "vue"
import {
  type AllDeviceSettings,
  DeviceUISettings,
  DeviceUISettingsDTO,
  SensorAndChannelSettings,
  type SystemOverviewOptions,
  UISettingsDTO
} from "@/models/UISettings"
import type {UID} from "@/models/Device"
import {Device} from "@/models/Device"
import setDefaultSensorAndChannelColors from "@/stores/DeviceColorCreator"
import {useDeviceStore} from "@/stores/DeviceStore"
import type {AllDaemonDeviceSettings} from "@/models/DaemonSettings"
import {
  DaemonDeviceSettings,
  DeviceSettingReadDTO,
  DeviceSettingWriteLcdDTO,
  DeviceSettingWriteLightingDTO,
  DeviceSettingWriteManualDTO,
  DeviceSettingWriteProfileDTO,
  DeviceSettingWritePWMModeDTO,
} from "@/models/DaemonSettings"
import {useToast} from "primevue/usetoast"

export const useSettingsStore =
    defineStore('settings', () => {

      const toast = useToast()

      const predefinedColorOptions: Ref<Array<string>> = ref([ // todo: used color history
        '#FFFFFF',
        '#000000',
        '#FF0000',
        '#FFFF00',
        '#00FF00',
        '#FF00FF',
        '#00FFFF',
        '#0000FF',
      ])

      const functions: Ref<Array<Function>> = ref([])

      const profiles: Ref<Array<Profile>> = ref([])

      const allUIDeviceSettings: Ref<AllDeviceSettings> = ref(new Map<UID, DeviceUISettings>())

      const allDaemonDeviceSettings: Ref<AllDaemonDeviceSettings> = ref(new Map<UID, DaemonDeviceSettings>())

      const systemOverviewOptions: SystemOverviewOptions = reactive({
        selectedTimeRange: {name: '1 min', seconds: 60},
        selectedChartType: 'TimeChart',
      })

      /**
       * This is used to help track various updates that should trigger a refresh of data for the sidebar menu.
       * Currently used to watch for changes indirectly.
       */
      function sidebarMenuUpdate(): void {
        console.debug('Sidebar Menu Update Triggered')
      }

      async function initializeSettings(allDevicesIter: IterableIterator<Device>): Promise<void> {
        // set defaults for all devices:
        const allDevices = [...allDevicesIter]
        for (const device of allDevices) {
          const deviceSettings = new DeviceUISettings()
          // Prepare all base settings:
          for (const temp of device.status.temps) {
            deviceSettings.sensorsAndChannels.setValue(temp.name, new SensorAndChannelSettings())
          }
          for (const channel of device.status.channels) { // This gives us both "load" and "speed" channels
            deviceSettings.sensorsAndChannels.setValue(channel.name, new SensorAndChannelSettings())
          }
          if (device.info != null) {
            for (const [channelName, channelInfo] of device.info.channels.entries()) {
              if (channelInfo.lighting_modes.length > 0) {
                const settings = new SensorAndChannelSettings()
                deviceSettings.sensorsAndChannels.setValue(channelName, settings)
              } else if (channelInfo.lcd_modes.length > 0) {
                const settings = new SensorAndChannelSettings()
                deviceSettings.sensorsAndChannels.setValue(channelName, settings)
              }
            }
          }
          allUIDeviceSettings.value.set(device.uid, deviceSettings)
        }

        setDefaultSensorAndChannelColors(allDevices, allUIDeviceSettings.value)

        // load settings from persisted settings, overwriting those that are set
        const deviceStore = useDeviceStore()
        const uiSettings = await deviceStore.loadUiSettings()
        if (uiSettings.systemOverviewOptions != null) {
          systemOverviewOptions.selectedTimeRange = uiSettings.systemOverviewOptions.selectedTimeRange
          systemOverviewOptions.selectedChartType = uiSettings.systemOverviewOptions.selectedChartType
        }
        if (uiSettings.devices != null && uiSettings.deviceSettings != null
            && uiSettings.devices.length === uiSettings.deviceSettings.length) {
          for (const [i1, uid] of uiSettings.devices.entries()) {
            const deviceSettingsDto = uiSettings.deviceSettings[i1]
            const deviceSettings = new DeviceUISettings()
            deviceSettings.menuCollapsed = deviceSettingsDto.menuCollapsed
            deviceSettings.userName = deviceSettingsDto.userName
            if (deviceSettingsDto.names.length !== deviceSettingsDto.sensorAndChannelSettings.length) {
              continue
            }
            for (const [i2, name] of deviceSettingsDto.names.entries()) {
              deviceSettings.sensorsAndChannels.setValue(name, deviceSettingsDto.sensorAndChannelSettings[i2])
            }
            allUIDeviceSettings.value.set(uid, deviceSettings)
          }
        }
        setDisplayNames(allDevices, allUIDeviceSettings.value)
        await loadDaemonDeviceSettings()

        await loadFunctions()
        await loadProfiles()

        startWatchingToSaveChanges()
      }

      function setDisplayNames(devices: Array<Device>, deviceSettings: Map<UID, DeviceUISettings>): void {
        const deviceStore = useDeviceStore()
        for (const device of devices) {
          const settings = deviceSettings.get(device.uid)!
          settings.displayName = device.nameShort
          if (device.status_history.length) {
            for (const channelStatus of device.status.channels) {
              const isFanOrPumpChannel = channelStatus.name.includes('fan') || channelStatus.name.includes('pump')
              settings.sensorsAndChannels.getValue(channelStatus.name).displayName =
                  isFanOrPumpChannel ? deviceStore.toTitleCase(channelStatus.name) : channelStatus.name
            }
            for (const tempStatus of device.status.temps) {
              settings.sensorsAndChannels.getValue(tempStatus.name).displayName = tempStatus.frontend_name
            }
          }
          if (device.info != null) {
            for (const [channelName, channelInfo] of device.info.channels.entries()) {
              if (channelInfo.lighting_modes.length > 0) {
                settings.sensorsAndChannels.getValue(channelName).displayName = deviceStore.toTitleCase(channelName)
              } else if (channelInfo.lcd_modes.length > 0) {
                settings.sensorsAndChannels.getValue(channelName).displayName = channelName.toUpperCase()
              }
            }
          }
        }
      }

      async function loadDaemonDeviceSettings(deviceUID: string | undefined = undefined): Promise<void> {
        const deviceStore = useDeviceStore()
        // allDevices() is used to handle cases where a device may be hidden and no longer available
        for (const device of deviceStore.allDevices()) { // we could load these in parallel, but it's anyway really fast
          if (deviceUID != null && device.uid !== deviceUID) {
            continue
          }
          const deviceSettingsDTO = await deviceStore.loadDeviceSettings(device.uid)
          const deviceSettings = new DaemonDeviceSettings()
          deviceSettingsDTO.settings.forEach(
              (setting: DeviceSettingReadDTO) => deviceSettings.settings.set(setting.channel_name, setting)
          )
          allDaemonDeviceSettings.value.set(device.uid, deviceSettings)
        }
      }

      /**
       * Loads all the Functions from the daemon. The default Function must be included.
       * These should be loaded before Profiles, as Profiles reference associated Functions.
       */
      async function loadFunctions(): Promise<void> {
        const functionsDTO = await useDeviceStore().loadFunctions()
        if (functionsDTO.functions.find((fun: Function) => fun.uid === '0') == null) {
          throw new Error("Default Function not present in daemon Response. We should not continue.")
        }
        functions.value.length = 0
        functions.value = functionsDTO.functions
      }

      /**
       * Saves the Functions order ONLY to the daemon.
       */
      async function saveFunctionsOrder(): Promise<void> {
        console.debug("Saving Functions Order")
        const functionsDTO = new FunctionsDTO()
        functionsDTO.functions = functions.value
        await useDeviceStore().saveFunctionsOrder(functionsDTO)
      }

      async function saveFunction(functionUID: UID): Promise<void> {
        console.debug("Saving Function")
        const fun_to_save = functions.value.find(fun => fun.uid === functionUID)
        if (fun_to_save == null) {
          console.error("Function to save not found: " + functionUID)
          return
        }
        await useDeviceStore().saveFunction(fun_to_save)
      }

      async function updateFunction(functionUID: UID): Promise<boolean> {
        console.debug("Updating Function")
        const fun_to_update = functions.value.find(fun => fun.uid === functionUID)
        if (fun_to_update == null) {
          console.error("Function to update not found: " + functionUID)
          return false
        }
        return await useDeviceStore().updateFunction(fun_to_update)
      }

      async function deleteFunction(functionUID: UID): Promise<void> {
        console.debug("Deleting Function")
        await useDeviceStore().deleteFunction(functionUID)
      }

      /**
       * Loads all the Profiles from the daemon. The default Profile must be included.
       */
      async function loadProfiles(): Promise<void> {
        const deviceStore = useDeviceStore()
        const profilesDTO = await deviceStore.loadProfiles()
        if (profilesDTO.profiles.find((profile: Profile) => profile.uid === '0') == null) {
          throw new Error("Default Profile not present in daemon Response. We should not continue.")
        }
        profiles.value.length = 0
        profiles.value = profilesDTO.profiles
      }

      /**
       * Saves the Profiles Order ONLY to the daemon.
       */
      async function saveProfilesOrder(): Promise<void> {
        console.debug("Saving Profiles Order")
        const profilesDTO = new ProfilesDTO()
        profilesDTO.profiles = profiles.value
        await useDeviceStore().saveProfilesOrder(profilesDTO)
      }

      async function saveProfile(profileUID: UID): Promise<void> {
        console.debug("Saving Profile")
        const profile_to_save = profiles.value.find(profile => profile.uid === profileUID)
        if (profile_to_save == null) {
          console.error("Profile to save not found: " + profileUID)
          return
        }
        await useDeviceStore().saveProfile(profile_to_save)
      }

      async function updateProfile(profileUID: UID): Promise<boolean> {
        console.debug("Updating Profile")
        const profile_to_update = profiles.value.find(profile => profile.uid === profileUID)
        if (profile_to_update == null) {
          console.error("Profile to update not found: " + profileUID)
          return false
        }
        return await useDeviceStore().updateProfile(profile_to_update)
      }

      async function deleteProfile(profileUID: UID): Promise<void> {
        console.debug("Deleting Profile")
        await useDeviceStore().deleteProfile(profileUID)
      }

      /**
       * This needs to be called after everything is initialized and setup, then we can sync all UI settings automatically.
       */
      function startWatchingToSaveChanges() {
        watch([allUIDeviceSettings.value, systemOverviewOptions], async () => {
          console.debug("Saving UI Settings")
          const deviceStore = useDeviceStore()
          const uiSettings = new UISettingsDTO()
          for (const [uid, deviceSettings] of allUIDeviceSettings.value) {
            uiSettings.devices?.push(toRaw(uid))
            const deviceSettingsDto = new DeviceUISettingsDTO()
            deviceSettingsDto.menuCollapsed = deviceSettings.menuCollapsed
            deviceSettingsDto.userName = deviceSettings.userName
            deviceSettings.sensorsAndChannels.forEach((name, sensorAndChannelSettings) => {
              deviceSettingsDto.names.push(name)
              deviceSettingsDto.sensorAndChannelSettings.push(sensorAndChannelSettings)
            })
            uiSettings.deviceSettings?.push(deviceSettingsDto)
          }
          uiSettings.systemOverviewOptions = systemOverviewOptions
          await deviceStore.saveUiSettings(uiSettings)
        })
      }

      async function handleSaveDeviceSettingResponse(deviceUID: UID, successful: boolean): Promise<void> {
        if (successful) {
          await loadDaemonDeviceSettings(deviceUID)
          toast.add({severity: 'success', summary: 'Success', detail: 'Settings successfully updated and applied to the device', life: 3000})
        } else {
          toast.add({severity: 'error', summary: 'Error', detail: 'There was an error when attempting to apply these settings', life: 3000})
        }
        console.debug('Daemon Settings Saved')
      }

      async function saveDaemonDeviceSettingManual(
          deviceUID: UID,
          channelName: string,
          setting: DeviceSettingWriteManualDTO
      ): Promise<void> {
        const deviceStore = useDeviceStore()
        const successful = await deviceStore.saveDeviceSettingManual(deviceUID, channelName, setting)
        await handleSaveDeviceSettingResponse(deviceUID, successful)
      }

      async function saveDaemonDeviceSettingProfile(
          deviceUID: UID,
          channelName: string,
          setting: DeviceSettingWriteProfileDTO
      ): Promise<void> {
        const deviceStore = useDeviceStore()
        const successful = await deviceStore.saveDeviceSettingProfile(deviceUID, channelName, setting)
        await handleSaveDeviceSettingResponse(deviceUID, successful)
      }

      async function saveDaemonDeviceSettingLcd(
          deviceUID: UID,
          channelName: string,
          setting: DeviceSettingWriteLcdDTO
      ): Promise<void> {
        const deviceStore = useDeviceStore()
        const successful = await deviceStore.saveDeviceSettingLcd(deviceUID, channelName, setting)
        await handleSaveDeviceSettingResponse(deviceUID, successful)
      }

      async function saveDaemonDeviceSettingLighting(
          deviceUID: UID,
          channelName: string,
          setting: DeviceSettingWriteLightingDTO
      ): Promise<void> {
        const deviceStore = useDeviceStore()
        const successful = await deviceStore.saveDeviceSettingLighting(deviceUID, channelName, setting)
        await handleSaveDeviceSettingResponse(deviceUID, successful)
      }

      async function saveDaemonDeviceSettingPWM(
          deviceUID: UID,
          channelName: string,
          setting: DeviceSettingWritePWMModeDTO
      ): Promise<void> {
        const deviceStore = useDeviceStore()
        const successful = await deviceStore.saveDeviceSettingPWM(deviceUID, channelName, setting)
        await handleSaveDeviceSettingResponse(deviceUID, successful)
      }

      async function saveDaemonDeviceSettingReset(
          deviceUID: UID,
          channelName: string,
      ): Promise<void> {
        const deviceStore = useDeviceStore()
        const successful = await deviceStore.saveDeviceSettingReset(deviceUID, channelName)
        await handleSaveDeviceSettingResponse(deviceUID, successful)
      }


      console.debug(`Settings Store created`)
      return {
        initializeSettings, predefinedColorOptions, profiles, functions, allUIDeviceSettings, sidebarMenuUpdate,
        systemOverviewOptions, allDaemonDeviceSettings,
        saveDaemonDeviceSettingManual, saveDaemonDeviceSettingProfile, saveDaemonDeviceSettingLcd,
        saveDaemonDeviceSettingLighting, saveDaemonDeviceSettingPWM, saveDaemonDeviceSettingReset,
        saveFunctionsOrder, saveFunction, updateFunction, deleteFunction,
        saveProfilesOrder, saveProfile, updateProfile, deleteProfile,
      }
    })
