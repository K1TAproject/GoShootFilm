package com.k1taproject.goshootfilm.data

import android.content.Context
import android.os.Build
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.longPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map

private val Context.dataStore by preferencesDataStore(name = "gsfmobile_settings")

data class AppSettings(
    val serverUrl: String = "",
    val serverFingerprint: String = "",
    val deviceId: String = "",
    val deviceName: String = defaultDeviceName(),
    val lastSync: Long = 0,
    val cursor: Long = 0,
    val wifiOnly: Boolean = true,
    val cacheLimitMb: Int = 5120,
    val pendingServerUrl: String = "",
    val pendingFingerprint: String = "",
)

private fun defaultDeviceName(): String {
    val manufacturer = Build.MANUFACTURER.orEmpty().trim()
    val model = Build.MODEL.orEmpty().trim()
    val value = listOf(manufacturer, model).filter { it.isNotBlank() }.joinToString(" ")
    return value.ifBlank { "Android" }.take(80)
}

class SettingsStore(private val context: Context) {
    private object Keys {
        val serverUrl = stringPreferencesKey("server_url")
        val serverFingerprint = stringPreferencesKey("server_fingerprint")
        val deviceId = stringPreferencesKey("device_id")
        val deviceName = stringPreferencesKey("device_name")
        val lastSync = longPreferencesKey("last_sync")
        val cursor = longPreferencesKey("cursor")
        val wifiOnly = booleanPreferencesKey("wifi_only")
        val cacheLimitMb = intPreferencesKey("cache_limit_mb")
        val pendingServerUrl = stringPreferencesKey("pending_server_url")
        val pendingFingerprint = stringPreferencesKey("pending_fingerprint")
    }

    val settings: Flow<AppSettings> = context.dataStore.data.map { prefs -> prefs.toSettings() }

    suspend fun current(): AppSettings = context.dataStore.data.first().toSettings()

    suspend fun setPairing(
        serverUrl: String,
        fingerprint: String,
        deviceId: String,
        resetCursor: Boolean = false,
    ) {
        context.dataStore.edit {
            it[Keys.serverUrl] = serverUrl.trimEnd('/')
            it[Keys.serverFingerprint] = fingerprint
            it[Keys.deviceId] = deviceId
            if (resetCursor) it[Keys.cursor] = 0L
            it.remove(Keys.pendingServerUrl)
            it.remove(Keys.pendingFingerprint)
        }
    }

    suspend fun setPendingPairing(serverUrl: String, fingerprint: String) {
        context.dataStore.edit {
            it[Keys.pendingServerUrl] = serverUrl.trimEnd('/')
            it[Keys.pendingFingerprint] = fingerprint
        }
    }

    suspend fun clearPendingPairing() {
        context.dataStore.edit {
            it.remove(Keys.pendingServerUrl)
            it.remove(Keys.pendingFingerprint)
        }
    }

    suspend fun clearPairing() {
        context.dataStore.edit {
            it.remove(Keys.serverUrl)
            it.remove(Keys.serverFingerprint)
            it.remove(Keys.deviceId)
            it.remove(Keys.pendingServerUrl)
            it.remove(Keys.pendingFingerprint)
            it[Keys.cursor] = 0L
            it[Keys.lastSync] = 0L
        }
    }

    suspend fun setCursor(cursor: Long) {
        context.dataStore.edit { it[Keys.cursor] = cursor }
    }

    suspend fun setLastSync(epochMillis: Long) {
        context.dataStore.edit { it[Keys.lastSync] = epochMillis }
    }

    suspend fun setWifiOnly(value: Boolean) {
        context.dataStore.edit { it[Keys.wifiOnly] = value }
    }

    suspend fun setCacheLimitMb(value: Int) {
        context.dataStore.edit { it[Keys.cacheLimitMb] = value }
    }

    private fun Preferences.toSettings(): AppSettings = AppSettings(
        serverUrl = this[Keys.serverUrl].orEmpty(),
        serverFingerprint = this[Keys.serverFingerprint].orEmpty(),
        deviceId = this[Keys.deviceId].orEmpty(),
        deviceName = this[Keys.deviceName] ?: defaultDeviceName(),
        lastSync = this[Keys.lastSync] ?: 0L,
        cursor = this[Keys.cursor] ?: 0L,
        wifiOnly = this[Keys.wifiOnly] ?: true,
        cacheLimitMb = this[Keys.cacheLimitMb] ?: 5120,
        pendingServerUrl = this[Keys.pendingServerUrl].orEmpty(),
        pendingFingerprint = this[Keys.pendingFingerprint].orEmpty(),
    )
}
