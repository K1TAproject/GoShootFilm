package com.k1taproject.goshootfilm.data

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull

@Entity(
    tableName = "entities",
    primaryKeys = ["entityType", "syncId"],
    indices = [Index("entityType"), Index("entityType", "syncId")],
)
data class EntityRecord(
    val entityType: String,
    val syncId: String,
    val revision: Long,
    val deleted: Boolean,
    val dataJson: String,
    val updatedAt: Long,
)

@Entity(
    tableName = "assets",
    primaryKeys = ["id"],
    indices = [Index("entityType", "entitySyncId"), Index("assetType")],
)
data class AssetRecord(
    val id: String,
    val entityType: String,
    val entitySyncId: String,
    val assetType: String,
    val version: String,
    val mimeType: String,
    val byteSize: Long?,
    val sha256: String?,
    val available: Boolean,
    val localPath: String?,
    val lastAccess: Long,
)

@Entity(tableName = "meta", primaryKeys = ["key"])
data class MetaRecord(
    val key: String,
    val value: String,
)

val AppJson = Json {
    ignoreUnknownKeys = true
    isLenient = true
    explicitNulls = false
}

fun EntityRecord.obj(): JsonObject =
    runCatching { AppJson.parseToJsonElement(dataJson).jsonObject }.getOrElse { JsonObject(emptyMap()) }

fun JsonObject.text(key: String): String =
    this[key]?.jsonPrimitive?.contentOrNull.orEmpty()

fun JsonObject.optionalText(key: String): String? =
    this[key]?.jsonPrimitive?.contentOrNull?.takeIf { it.isNotBlank() }

fun JsonObject.int(key: String): Int? =
    this[key]?.jsonPrimitive?.intOrNull

fun JsonObject.long(key: String): Long? =
    this[key]?.jsonPrimitive?.longOrNull

fun JsonObject.bool(key: String): Boolean =
    this[key]?.jsonPrimitive?.booleanOrNull ?: false

fun EntityRecord.displayTitle(): String {
    val value = obj()
    return when (entityType) {
        "camera", "equipment" -> listOf(value.text("brand"), value.text("model"))
            .filter { it.isNotBlank() }
            .joinToString(" ")
        "film" -> listOf(value.text("brand"), value.text("name"))
            .filter { it.isNotBlank() }
            .joinToString(" ")
        "roll" -> buildString {
            append(value.text("cameraInfo").ifBlank { "Roll" })
            val index = value.int("index")
            if (index != null) append(" · #$index")
        }
        "digital_album" -> value.text("title").ifBlank { "Album" }
        "film_photo" -> value.text("displayName").ifBlank { "Frame" }
        "digital_photo" -> value.text("pairingKey").ifBlank { "Photo" }
        else -> syncId
    }
}
