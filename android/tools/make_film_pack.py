from pathlib import Path
from PIL import Image, ImageDraw, ImageFont, ImageOps

ROOT = Path(r"D:\GoShootFilm")
OUT = ROOT / "android" / "app" / "src" / "main" / "assets"
SRC = ROOT / "public" / "film-stocks"
(OUT / "film-stocks").mkdir(parents=True, exist_ok=True)
(OUT / "film-thumbs").mkdir(parents=True, exist_ok=True)

bg = (16, 21, 29, 255)
accent = (155, 135, 245, 255)

for image_path in sorted(SRC.glob("*.png")):
    image = Image.open(image_path).convert("RGBA")
    canvas = Image.new("RGBA", image.size, bg)
    canvas.alpha_composite(image)
    image = canvas.convert("RGB")
    for subdir, size in (("film-stocks", (820, 400)), ("film-thumbs", (410, 200))):
        fitted = ImageOps.fit(image, size, method=Image.Resampling.LANCZOS, centering=(0.5, 0.5))
        target = OUT / subdir / f"{image_path.stem}.webp"
        fitted.save(target, "WEBP", quality=82, method=4)

for subdir, size in (("film-stocks", (820, 400)), ("film-thumbs", (410, 200))):
    placeholder = Image.new("RGB", size, (16, 21, 29))
    draw = ImageDraw.Draw(placeholder)
    draw.rectangle([0, 0, size[0], 6], fill=accent)
    draw.text((24, size[1] // 2 - 10), "GSFMobile film stock", fill=(200, 205, 220))
    placeholder.save(OUT / subdir / "placeholder.webp", "WEBP", quality=82, method=4)

(OUT / "film-catalog.csv").write_text(
    (ROOT / "src" / "data" / "film-catalog.csv").read_text(encoding="utf-8"),
    encoding="utf-8",
)
print("film stocks:", len(list((OUT / 'film-stocks').glob('*.webp'))))
print("film thumbs:", len(list((OUT / 'film-thumbs').glob('*.webp'))))
print("size MB:", round(sum(p.stat().st_size for p in OUT.rglob('*.webp')) / 1024 / 1024, 2))
