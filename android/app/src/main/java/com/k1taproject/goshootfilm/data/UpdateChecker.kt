package com.k1taproject.goshootfilm.data

import android.os.Build
import com.k1taproject.goshootfilm.BuildConfig
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

data class UpdateInfo(
    val versionCode: Int,
    val versionName: String,
    val notes: String,
    val apkUrl: String,
    val releaseUrl: String,
    val sha256: String,
)

class UpdateChecker {
    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(20, TimeUnit.SECONDS)
        .build()

    fun check(currentVersionCode: Int): UpdateInfo? {
        val releases = getJson("$API/releases?per_page=20") as? kotlinx.serialization.json.JsonArray
            ?: return null
        for (element in releases) {
            val release = element.jsonObject
            if (release["draft"]?.jsonPrimitive?.booleanOrNull == true) continue
            if (release["prerelease"]?.jsonPrimitive?.booleanOrNull == true) continue
            val assets = release["assets"]?.jsonArray ?: continue
            val metadataAsset = assets.firstOrNull {
                it.jsonObject.text("name") == "mobile-update.json"
            }?.jsonObject ?: continue
            val metadataUrl = metadataAsset.text("browser_download_url")
            if (metadataUrl.isBlank()) continue
            val metadata = getJson(metadataUrl) as? JsonObject ?: continue
            if (metadata.text("applicationId") != BuildConfig.APPLICATION_ID) continue
            if ((metadata.int("minSdk") ?: 0) > Build.VERSION.SDK_INT) continue
            val versionCode = metadata.int("versionCode") ?: continue
            if (versionCode <= currentVersionCode) continue
            val apkName = metadata.text("apkName")
            if (apkName.isBlank()) continue
            val apkAsset = assets.firstOrNull {
                it.jsonObject.text("name") == apkName
            }?.jsonObject ?: continue
            val apkUrl = apkAsset.text("browser_download_url")
            if (apkUrl.isBlank()) continue
            return UpdateInfo(
                versionCode = versionCode,
                versionName = metadata.text("versionName"),
                notes = release.text("body").ifBlank { "GSFMobile 新版本" },
                apkUrl = apkUrl,
                releaseUrl = release.text("html_url"),
                sha256 = metadata.text("sha256"),
            )
        }
        return null
    }

    private fun getJson(url: String): Any? {
        val request = Request.Builder()
            .url(url)
            .header("User-Agent", "GSFMobile-Android")
            .header("Accept", "application/vnd.github+json")
            .get()
            .build()
        client.newCall(request).execute().use { response ->
            if (!response.isSuccessful) return null
            val text = response.body?.string().orEmpty()
            return runCatching { AppJson.parseToJsonElement(text) }.getOrNull()
        }
    }

    companion object {
        private const val API = "https://api.github.com/repos/K1TAproject/GoShootFilm"
    }
}
