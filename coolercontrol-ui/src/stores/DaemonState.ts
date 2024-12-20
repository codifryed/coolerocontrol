/*
 * CoolerControl - monitor and control your cooling and other devices
 * Copyright (c) 2021-2024  Guy Boldon and contributors
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

import { defineStore } from 'pinia'
import { ref, Ref } from 'vue'
import { useDeviceStore } from '@/stores/DeviceStore.ts'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from 'primevue/usetoast'

export enum DaemonStatus {
    OK = 'Ok',
    WARN = 'Has Warnings',
    ERROR = 'Has Errors',
}

export const useDaemonState = defineStore('daemonState', () => {
    const toast = useToast()
    // Reactive properties ------------------------------------------------
    const systemName: Ref<string> = ref('Localhost')
    const warnings: Ref<number> = ref(0)
    const errors: Ref<number> = ref(0)
    const status: Ref<DaemonStatus> = ref(DaemonStatus.ERROR)
    const connected: Ref<boolean> = ref(false)
    const preDisconnectedStatus: Ref<DaemonStatus> = ref(DaemonStatus.ERROR)

    async function init(): Promise<void> {
        const deviceStore = useDeviceStore()
        const healthCheck = await deviceStore.health()
        systemName.value = healthCheck.system.name
        warnings.value = healthCheck.details.warnings
        errors.value = healthCheck.details.errors
        connected.value = true
        if (errors.value > 0) {
            await setStatus(DaemonStatus.ERROR)
        } else if (warnings.value > 0) {
            await setStatus(DaemonStatus.WARN)
        } else {
            await setStatus(DaemonStatus.OK)
        }
        if (deviceStore.isTauriApp()) {
            await invoke('connected_to_daemon', {
                daemonAddress: deviceStore.daemonClient.daemonURL,
                hasErrors: errors.value > 0,
            })
        }
    }

    async function setStatus(newStatus: DaemonStatus) {
        if (status.value === newStatus) return
        if (newStatus === DaemonStatus.ERROR) {
            toast.add({
                severity: 'error',
                summary: 'Daemon Errors',
                detail: 'The daemon logs contain errors. You should investigate.',
                life: 4000,
            })
        }
        status.value = newStatus
    }

    async function setConnected(isConnected: boolean): Promise<void> {
        if (connected.value === isConnected) return
        if (connected.value) {
            // disconnected
            preDisconnectedStatus.value = status.value
            toast.add({
                severity: 'error',
                summary: 'Daemon Disconnected',
                detail: 'Connection with the daemon has been lost',
                life: 4000,
            })
            status.value = DaemonStatus.ERROR
        } else {
            // re-connected
            status.value = preDisconnectedStatus.value
            toast.add({
                severity: 'success',
                summary: 'Daemon Connection Restored',
                detail: 'Connection with the daemon has been restored.',
                life: 4000,
            })
            const deviceStore = useDeviceStore()
            // re-load the logs in case the daemon has restarted
            await deviceStore.load_logs()
        }
        connected.value = isConnected
    }

    async function acknowledgeLogIssues(): Promise<void> {
        if (!connected.value) return
        status.value = DaemonStatus.OK
        const deviceStore = useDeviceStore()
        if (deviceStore.isTauriApp()) {
            await invoke('acknowledge_daemon_issues')
        }
    }

    console.debug(`Daemon State Store created`)
    return {
        init,
        setStatus,
        setConnected,
        acknowledgeLogIssues,
        systemName,
        warnings,
        errors,
        status,
    }
})
