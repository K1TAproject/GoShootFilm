package com.k1taproject.goshootfilm

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.k1taproject.goshootfilm.data.AppDatabase
import com.k1taproject.goshootfilm.data.AppSettings
import com.k1taproject.goshootfilm.data.AssetRecord
import com.k1taproject.goshootfilm.data.FilmCatalog
import com.k1taproject.goshootfilm.data.SecureStore
import com.k1taproject.goshootfilm.data.SettingsStore
import com.k1taproject.goshootfilm.data.SyncPhase
import com.k1taproject.goshootfilm.data.SyncRepository
import com.k1taproject.goshootfilm.data.UpdateChecker
import com.k1taproject.goshootfilm.data.UpdateInfo
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import java.io.File

sealed interface SyncState {
    data object Idle : SyncState
    data class Running(
        val message: String,
        val phase: SyncPhase = SyncPhase.Checking,
        val progress: Float? = null,
    ) : SyncState
    data class Done(val at: Long, val message: String = "同步完成") : SyncState
    data class Failed(val message: String) : SyncState
}

sealed interface PairState {
    data object Idle : PairState
    data class Waiting(val status: String) : PairState
    data object Paired : PairState
    data class Failed(val message: String) : PairState
}

sealed interface UpdateState {
    data object Idle : UpdateState
    data object Checking : UpdateState
    data class Available(val info: UpdateInfo) : UpdateState
    data object Latest : UpdateState
    data class Failed(val message: String) : UpdateState
}

class GsfViewModel(application: Application) : AndroidViewModel(application) {
    private val database = AppDatabase.get(application)
    private val settingsStore = SettingsStore(application)
    private val secureStore = SecureStore(application)
    val repository = SyncRepository(application, database, settingsStore, secureStore)
    val filmCatalog = FilmCatalog(application)
    private val updateChecker = UpdateChecker()

    val settings: StateFlow<AppSettings> = repository.settingsFlow().stateIn(
        viewModelScope,
        SharingStarted.WhileSubscribed(5_000),
        AppSettings(),
    )

    private val _syncState = MutableStateFlow<SyncState>(SyncState.Idle)
    val syncState: StateFlow<SyncState> = _syncState.asStateFlow()

    private val _pairState = MutableStateFlow<PairState>(PairState.Idle)
    val pairState: StateFlow<PairState> = _pairState.asStateFlow()

    private val _updateState = MutableStateFlow<UpdateState>(UpdateState.Idle)
    val updateState: StateFlow<UpdateState> = _updateState.asStateFlow()

    fun sync() {
        if (_syncState.value is SyncState.Running) return
        _syncState.value = SyncState.Running("准备同步…")
        viewModelScope.launch {
            try {
                var lastMessage = "同步完成"
                repository.sync { progress ->
                    lastMessage = progress.message
                    _syncState.value = SyncState.Running(
                        progress.message,
                        progress.phase,
                        progress.fraction,
                    )
                }
                _syncState.value = SyncState.Done(System.currentTimeMillis(), lastMessage)
            } catch (error: Exception) {
                _syncState.value = SyncState.Failed(error.message ?: "同步失败")
            }
        }
    }

    fun pairFromPayload(raw: String) {
        if (_pairState.value is PairState.Waiting) return
        _pairState.value = PairState.Waiting("正在提交配对请求…")
        viewModelScope.launch {
            try {
                val requestId = repository.beginPairing(raw)
                repository.waitForPairing(requestId) { status ->
                    _pairState.value = PairState.Waiting(
                        when (status) {
                            "pending" -> "等待电脑端批准…"
                            "approved" -> "已批准，正在保存凭证…"
                            else -> status
                        },
                    )
                }
                _pairState.value = PairState.Paired
                sync()
            } catch (error: Exception) {
                _pairState.value = PairState.Failed(error.message ?: "配对失败")
            }
        }
    }

    fun unpair() {
        viewModelScope.launch {
            repository.unpair()
            _pairState.value = PairState.Idle
        }
    }

    fun checkForUpdate() {
        if (_updateState.value is UpdateState.Checking) return
        _updateState.value = UpdateState.Checking
        viewModelScope.launch {
            try {
                val info = updateChecker.check(BuildConfig.VERSION_CODE)
                _updateState.value = if (info == null) UpdateState.Latest else UpdateState.Available(info)
            } catch (error: Exception) {
                _updateState.value = UpdateState.Failed(error.message ?: "检查更新失败")
            }
        }
    }

    fun cancelSync() {
        repository.cancelSync()
    }

    fun clearViewCache() {
        viewModelScope.launch {
            repository.clearViewCache()
        }
    }

    fun clearPinnedCache() {
        viewModelScope.launch {
            repository.clearPinnedEditOriginals()
        }
    }

    fun setWifiOnly(value: Boolean) {
        viewModelScope.launch {
            repository.setWifiOnly(value)
        }
    }

    fun setCacheLimit(value: Int) {
        viewModelScope.launch {
            repository.setCacheLimitMb(value)
        }
    }

    fun clearAllData() {
        viewModelScope.launch {
            repository.clearAllData()
            _syncState.value = SyncState.Idle
        }
    }

    suspend fun thumbnail(type: String, syncId: String): AssetRecord? =
        repository.thumbnail(type, syncId)

    suspend fun editOriginal(type: String, syncId: String): AssetRecord? =
        repository.editOriginal(type, syncId)

    suspend fun localEditFile(assetId: String): File? = repository.localEditFile(assetId)

    suspend fun downloadEditOriginal(
        assetId: String,
        onProgress: (Long, Long) -> Unit,
    ): File = repository.downloadEditOriginal(assetId, onProgress)

    suspend fun entities(type: String) = repository.entitiesNow(type)

    suspend fun entity(type: String, syncId: String) = repository.entity(type, syncId)

    suspend fun cachedBytes() = repository.cachedBytes()

    suspend fun cachedPinnedBytes() = repository.cachedPinnedBytes()

    fun consumePairError() {
        if (_pairState.value is PairState.Failed) _pairState.value = PairState.Idle
    }
}
