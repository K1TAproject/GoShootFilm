#!/usr/bin/env python3
"""Deterministically extend mapped film-stock image backgrounds.

The original image is pasted unchanged in the center. Only newly added side
areas are generated from mirrored edge strips, blur, and a seam gradient.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import math
import os
import tempfile
from pathlib import Path
from typing import Any

from PIL import Image, ImageFilter, ImageOps, ImageStat


SUPPORTED_SUFFIXES = {".png", ".jpg", ".jpeg", ".webp"}
MANIFEST_VERSION = 1


def file_hash(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def pixel_hash(image: Image.Image) -> str:
    return hashlib.sha256(image.convert("RGBA").tobytes()).hexdigest()


def read_catalog(path: Path) -> list[dict[str, str]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    if len(lines) < 3 or not lines[0].startswith("version;"):
        raise ValueError(f"Invalid film catalog: {path}")
    rows = list(csv.DictReader(io.StringIO("\n".join(lines[1:])), delimiter=";"))
    images = [row.get("image", "").strip() for row in rows]
    if any(not image for image in images) or len(images) != len(set(images)):
        raise ValueError("Catalog image paths are missing or duplicated")
    return rows


def load_manifest(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {"version": MANIFEST_VERSION, "images": {}}
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("version") != MANIFEST_VERSION or not isinstance(data.get("images"), dict):
        raise ValueError(f"Unsupported manifest: {path}")
    return data


def write_manifest(path: Path, manifest: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmp", dir=path.parent)
    try:
        with os.fdopen(handle, "w", encoding="utf-8", newline="\n") as stream:
            json.dump(manifest, stream, ensure_ascii=False, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary_name, path)
    except Exception:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass
        raise


def edge_complexity(image: Image.Image) -> float:
    rgb = image.convert("RGB")
    width, height = rgb.size
    strip_width = max(2, min(12, width // 30))
    combined = Image.new("RGB", (strip_width * 2, height))
    combined.paste(rgb.crop((0, 0, strip_width, height)), (0, 0))
    combined.paste(rgb.crop((width - strip_width, 0, width, height)), (strip_width, 0))
    return math.sqrt(sum(value * value for value in ImageStat.Stat(combined).stddev))


def side_extension(source: Image.Image, width: int, side: str) -> Image.Image:
    if width <= 0:
        return Image.new("RGBA", (0, source.height))
    edge_width = max(8, min(64, source.width // 24))
    if side == "left":
        strip = source.crop((0, 0, edge_width, source.height))
        edge_column = source.crop((0, 0, 1, source.height))
    else:
        strip = source.crop((source.width - edge_width, 0, source.width, source.height))
        edge_column = source.crop((source.width - 1, 0, source.width, source.height))

    extended = ImageOps.mirror(strip).resize((width, source.height), Image.Resampling.BICUBIC)
    blur_radius = max(4.0, min(18.0, width / 14.0))
    blurred = extended.filter(ImageFilter.GaussianBlur(blur_radius))
    direct_edge = edge_column.resize((width, source.height), Image.Resampling.BICUBIC)
    seam_width = min(width, max(16, source.width // 40))
    mask = Image.new("L", (width, source.height), 0)
    mask_pixels = mask.load()
    for offset in range(seam_width):
        strength = round(255 * (offset + 1) / seam_width)
        x = width - seam_width + offset if side == "left" else seam_width - 1 - offset
        for y in range(source.height):
            mask_pixels[x, y] = strength
    return Image.composite(direct_edge, blurred, mask)


def extend_background(source: Image.Image, target_width: int) -> tuple[Image.Image, int, int]:
    source = source.convert("RGBA")
    extra_width = target_width - source.width
    left_width = extra_width // 2
    right_width = extra_width - left_width
    output = Image.new("RGBA", (target_width, source.height))
    output.paste(side_extension(source, left_width, "left"), (0, 0))
    output.paste(source, (left_width, 0))
    output.paste(side_extension(source, right_width, "right"), (left_width + source.width, 0))
    return output, left_width, right_width


def save_and_validate(
    output: Image.Image,
    destination: Path,
    source_format: str,
    source_info: dict[str, Any],
    original_pixels: str,
    original_size: tuple[int, int],
    left_padding: int,
) -> None:
    handle, temporary_name = tempfile.mkstemp(
        prefix=f".{destination.stem}.", suffix=destination.suffix, dir=destination.parent
    )
    os.close(handle)
    temporary = Path(temporary_name)
    try:
        save_options: dict[str, Any] = {}
        for key in ("icc_profile", "exif", "dpi"):
            if key in source_info:
                save_options[key] = source_info[key]
        if source_format == "PNG":
            save_options["compress_level"] = 9
        output.save(temporary, format=source_format, **save_options)
        with Image.open(temporary) as verified:
            verified.load()
            if verified.size != output.size or verified.format != source_format:
                raise ValueError("Temporary output dimensions or format are invalid")
            width, height = original_size
            center = verified.convert("RGBA").crop(
                (left_padding, 0, left_padding + width, height)
            )
            if pixel_hash(center) != original_pixels:
                raise ValueError("Central original pixels changed during encoding")
        os.replace(temporary, destination)
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def parse_args() -> argparse.Namespace:
    project_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, default=project_root / "public" / "film-stocks")
    parser.add_argument("--catalog", type=Path, default=project_root / "src" / "data" / "film-catalog.csv")
    parser.add_argument("--manifest", type=Path, default=project_root / "tools" / "film_image_background_manifest.json")
    parser.add_argument("--target-ratio", type=float, default=2.05)
    parser.add_argument("--dry-run", action="store_true", help="Inspect without writing images or manifest")
    parser.add_argument("--force", action="store_true", help="Rebuild completed images from their preserved center pixels")
    parser.add_argument("--verbose", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not 1.5 < args.target_ratio < 4.0:
        raise ValueError("Target ratio must be between 1.5 and 4.0")
    rows = read_catalog(args.catalog.resolve())
    directory = args.directory.resolve()
    manifest_path = args.manifest.resolve()
    manifest = load_manifest(manifest_path)
    mapped_names = {row["image"].strip() for row in rows}
    supported_files = {
        path.name for path in directory.iterdir() if path.is_file() and path.suffix.lower() in SUPPORTED_SUFFIXES
    }
    unrelated = sorted(supported_files - mapped_names)

    checked = processed = skipped = failed = 0
    results: list[dict[str, Any]] = []
    for row in rows:
        name = row["image"].strip()
        path = directory / name
        checked += 1
        try:
            if path.suffix.lower() not in SUPPORTED_SUFFIXES or not path.is_file():
                raise ValueError("Mapped image is missing or unsupported")
            current_file_hash = file_hash(path)
            entry = manifest["images"].get(name)
            with Image.open(path) as opened:
                opened.load()
                current_format = opened.format
                current_info = dict(opened.info)
                current = opened.convert("RGBA")
            before_size = current.size

            if entry and entry.get("output_sha256") == current_file_hash:
                if not args.force:
                    skipped += 1
                    results.append({"file": name, "before": before_size, "after": before_size, "status": "skip-manifest"})
                    continue
                original_width, original_height = entry["original_size"]
                left_padding = entry["left_padding"]
                original = current.crop(
                    (left_padding, 0, left_padding + original_width, original_height)
                )
                if pixel_hash(original) != entry["original_pixel_sha256"]:
                    raise ValueError("Cannot safely reconstruct original center pixels for --force")
                original_file_hash = entry["original_sha256"]
                original_pixels = entry["original_pixel_sha256"]
                original_size = (original_width, original_height)
            else:
                original = current
                original_file_hash = current_file_hash
                original_pixels = pixel_hash(original)
                original_size = original.size

            target_width = math.ceil(original.height * args.target_ratio)
            if target_width <= original.width:
                skipped += 1
                results.append({"file": name, "before": before_size, "after": before_size, "status": "skip-ratio"})
                continue
            complexity = edge_complexity(original)
            if complexity > 40.0:
                raise ValueError(f"Edge background is too complex for safe extension ({complexity:.2f})")
            target_size = (target_width, original.height)
            results.append({"file": name, "before": original_size, "after": target_size, "status": "would-process" if args.dry_run else "processed"})
            if args.dry_run:
                processed += 1
                continue

            output, left_padding, right_padding = extend_background(original, target_width)
            save_and_validate(
                output,
                path,
                current_format or "PNG",
                current_info,
                original_pixels,
                original_size,
                left_padding,
            )
            output_hash = file_hash(path)
            manifest["images"][name] = {
                "original_size": list(original_size),
                "target_size": list(target_size),
                "target_ratio": args.target_ratio,
                "left_padding": left_padding,
                "right_padding": right_padding,
                "original_sha256": original_file_hash,
                "original_pixel_sha256": original_pixels,
                "output_sha256": output_hash,
                "edge_complexity": round(complexity, 4),
            }
            write_manifest(manifest_path, manifest)
            processed += 1
        except Exception as error:  # Continue auditing other files while preserving valid originals.
            failed += 1
            results.append({"file": name, "status": "failed", "reason": str(error)})

    if args.verbose:
        for result in results:
            print(json.dumps(result, ensure_ascii=False))
    summary = {
        "checked": checked,
        "processed" if not args.dry_run else "would_process": processed,
        "skipped": skipped,
        "failed": failed,
        "unrelated_supported_files": unrelated,
        "target_ratio": args.target_ratio,
        "dry_run": args.dry_run,
    }
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
