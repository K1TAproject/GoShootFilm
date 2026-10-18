package com.k1taproject.goshootfilm.data

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put
import okhttp3.HttpUrl
import okhttp3.HttpUrl.Companion.toHttpUrl
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import java.io.File
import java.io.FileOutputStream
import java.security.MessageDigest
import java.util.concurrent.TimeUnit

class ApiException(
    val status: Int,
    override val message: String,
) : Exception(message)

data class HealthInfo(
    val appVersion: String,
    val protocolVersion: Int,
    val fingerprint: String,
)

data class PairRequestResult(
    val requestId: String,
    val expiresAt: Long,
)

data class PairStatus(
    val status: String,
    val requestId: String? = null,
    val credential: String? = null,
    val deviceId: String? = null,
    val deviceName: String? = null,
    val message: String? = null,
)

data class DownloadResult(
    val path: String,
    val sha256: String,
    val byteSize: Long,
)

class ApiClient {
    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(5, TimeUnit.MINUTES)
        .writeTimeout(30, TimeUnit.SECONDS)
        .build()

    fun health(baseUrl: String): HealthInfo {
        val json = getJson("$baseUrl/api/v1/health")
        return HealthInfo(
            appVersion = json.text("appVersion"),
            protocolVersion = json.int("protocolVersion") ?: 0,
            fingerprint = json.text("serverFingerprint"),
        )
    }

    fun pairRequest(baseUrl: String, token: String, deviceName: String): PairRequestResult {
        requirePrivateHost("$baseUrl/api/v1/pair/request")
        val body = buildJsonObject {
            put("token", token)
            put("deviceName", deviceName)
            put("platform", "android")
            put("protocolVersion", 1)
        }
        val request = Request.Builder()
            .url("$baseUrl/api/v1/pair/request")
            .post(body.toString().toRequestBody(JSON))
            .build()
        val json = executeJson(request)
        return PairRequestResult(
            requestId = json.text("requestId"),
            expiresAt = json.long("expiresAt") ?: 0,
        )
    }

    fun pairStatus(baseUrl: String, requestId: String): PairStatus {
        val json = getJson("$baseUrl/api/v1/pair/status/$requestId")
        return PairStatus(
            status = json.text("status"),
            requestId = json.optionalText("requestId"),
            credential = json.optionalText("credential"),
            deviceId = json.optionalText("deviceId"),
            deviceName = json.optionalText("deviceName"),
            message = json.optionalText("message"),
        )
    }

    fun changes(baseUrl: String, credential: String, cursor: Long, limit: Int = 200): JsonObject {
        requirePrivateHost(baseUrl)
        val url = baseUrl.trimEnd('/').toHttpUrl().newBuilder()
            .addPathSegments("api/v1/sync/changes")
            .addQueryParameter("cursor", cursor.toString())
            .addQueryParameter("limit", limit.toString())
            .build()
        val request = Request.Builder()
            .url(url)
            .header("Authorization", "Bearer $credential")
            .get()
            .build()
        return executeJson(request)
    }

    fun downloadThumbnail(
        baseUrl: String,
        credential: String,
        assetId: String,
        destination: File,
        expectedSha256: String?,
        onProgress: (Long, Long) -> Unit,
    ): DownloadResult = download(
        "$baseUrl/api/v1/resources/$assetId",
        credential,
        destination,
        expectedSha256,
        onProgress,
    )

    fun downloadEditOriginal(
        baseUrl: String,
        credential: String,
        assetId: String,
        destination: File,
        expectedSha256: String?,
        onProgress: (Long, Long) -> Unit,
    ): DownloadResult = download(
        "$baseUrl/api/v1/edit-originals/$assetId",
        credential,
        destination,
        expectedSha256,
        onProgress,
    )

    private fun download(
        url: String,
        credential: String,
        destination: File,
        expectedSha256: String?,
        onProgress: (Long, Long) -> Unit,
    ): DownloadResult {
        requirePrivateHost(url)
        val request = Request.Builder()
            .url(url)
            .header("Authorization", "Bearer $credential")
            .get()
            .build()
        client.newCall(request).execute().use { response ->
            if (!response.isSuccessful) throw response.toApiException()
            val body = response.body ?: throw ApiException(response.code, "响应为空")
            val total = body.contentLength().coerceAtLeast(0)
            val parent = destination.parentFile ?: throw ApiException(0, "缓存目录无效")
            if (!parent.exists() && !parent.mkdirs()) throw ApiException(0, "无法创建缓存目录")
            val temp = File(parent, destination.name + ".part")
            var downloaded = 0L
            val digest = MessageDigest.getInstance("SHA-256")
            body.byteStream().use { input ->
                FileOutputStream(temp).use { output ->
                    val buffer = ByteArray(64 * 1024)
                    while (true) {
                        val read = input.read(buffer)
                        if (read <= 0) break
                        output.write(buffer, 0, read)
                        digest.update(buffer, 0, read)
                        downloaded += read
                        onProgress(downloaded, total)
                    }
                    output.fd.sync()
                }
            }
            val hash = digest.digest().joinToString("") { "%02x".format(it) }
            val expected = expectedSha256?.takeIf { it.isNotBlank() }
                ?: response.header("x-goshootfilm-sha256")
            if (!expected.isNullOrBlank() && !expected.equals(hash, ignoreCase = true)) {
                temp.delete()
                throw ApiException(response.code, "图片校验失败，请重试")
            }
            if (destination.exists()) destination.delete()
            if (!temp.renameTo(destination)) {
                temp.copyTo(destination, overwrite = true)
                temp.delete()
            }
            return DownloadResult(
                path = destination.absolutePath,
                sha256 = hash,
                byteSize = downloaded,
            )
        }
    }

    private fun getJson(url: String): JsonObject {
        requirePrivateHost(url)
        val request = Request.Builder().url(url).get().build()
        return executeJson(request)
    }

    private fun executeJson(request: Request): JsonObject {
        client.newCall(request).execute().use { response ->
            if (!response.isSuccessful) throw response.toApiException()
            val text = response.body?.string().orEmpty()
            return runCatching { AppJson.parseToJsonElement(text).jsonObject }
                .getOrElse { throw ApiException(response.code, "服务器返回格式不正确") }
        }
    }

    private fun Response.toApiException(): ApiException {
        val text = body?.string().orEmpty()
        val parsed = runCatching { AppJson.parseToJsonElement(text).jsonObject }.getOrNull()
        val message = parsed?.optionalText("message")
            ?: parsed?.optionalText("error")
            ?: when (code) {
                401 -> "设备已被撤销，请重新配对"
                404 -> "资源不存在"
                410 -> "二维码已过期或被使用"
                426 -> "手机端与电脑端协议不兼容"
                else -> "请求失败（HTTP $code）"
            }
        return ApiException(code, message)
    }

    private fun requirePrivateHost(url: String) {
        val parsed = url.toHttpUrlOrNull() ?: throw ApiException(0, "服务地址无效")
        if (parsed.scheme != "http" && parsed.scheme != "https") {
            throw ApiException(0, "只支持局域网 HTTP 服务")
        }
        val host = parsed.host
        val second = host.split(".").getOrNull(1)?.toIntOrNull()
        val private = host == "localhost" ||
            host == "::1" ||
            host.startsWith("127.") ||
            host.startsWith("10.") ||
            host.startsWith("192.168.") ||
            (host.startsWith("172.") && second != null && second in 16..31)
        if (!private) throw ApiException(0, "只允许连接局域网地址")
    }

    companion object {
        private val JSON = "application/json; charset=utf-8".toMediaType()
    }
}
