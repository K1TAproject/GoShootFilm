CREATE TEMP TABLE official_films (
    brand TEXT NOT NULL,
    name TEXT NOT NULL,
    iso INTEGER NOT NULL,
    type TEXT NOT NULL
);

INSERT INTO official_films (brand, name, iso, type) VALUES
    ('Foma', 'FOMAPAN 100 Classic Black & White Negative Film', 100, 'B&W'),
    ('Foma', 'FOMAPAN 200 Creative Black & White Negative Film', 200, 'B&W'),
    ('Foma', 'FOMAPAN 400 Action Black & White Negative Film', 400, 'B&W'),
    ('Foma', 'FOMAPAN R 100 Black & White Reversal Film', 100, 'B&W'),
    ('Foma', 'RETROPAN 320 soft Black & White Negative Film', 320, 'B&W'),
    ('Foma', 'FOMA Ortho 400 Black & White Negative Film', 400, 'B&W'),
    ('Fujifilm', 'FUJICOLOR 100 Color Negative Film', 100, 'Color Negative'),
    ('Fujifilm', 'FUJIFILM 200 Color Negative Film', 200, 'Color Negative'),
    ('Fujifilm', 'FUJIFILM 400 Color Negative Film', 400, 'Color Negative'),
    ('Fujifilm', 'FUJICOLOR PRO 400H Professional Color Negative Film', 400, 'Color Negative'),
    ('Fujifilm', 'FUJICOLOR SUPERIA X-TRA 400 Color Negative Film', 400, 'Color Negative'),
    ('Fujifilm', 'FUJICOLOR SUPERIA PREMIUM 400 Color Negative Film', 400, 'Color Negative'),
    ('Fujifilm', 'NEOPAN 100 ACROS II Black & White Negative Film', 100, 'B&W'),
    ('Fujifilm', 'FUJICHROME PROVIA 100F Professional Color Reversal Film', 100, 'Slide'),
    ('Fujifilm', 'FUJICHROME Velvia 50 Professional Color Reversal Film', 50, 'Slide'),
    ('Fujifilm', 'FUJICHROME Velvia 100 Professional Color Reversal Film', 100, 'Slide'),
    ('Kodak', 'KODAK PROFESSIONAL EKTACHROME E100 Color Reversal Film', 100, 'Slide'),
    ('Kodak', 'EASTMAN DOUBLE-X Black & White Negative Film 5222', 250, 'B&W'),
    ('Kodak', 'KODAK EKTACHROME 100D Color Reversal Film 5294', 100, 'Slide'),
    ('Kodak', 'KODAK VISION3 50D Color Negative Film 5203', 50, 'Color Negative'),
    ('Kodak', 'KODAK VISION3 250D Color Negative Film 5207', 250, 'Color Negative'),
    ('Kodak', 'KODAK VISION3 200T Color Negative Film 5213', 200, 'Color Negative'),
    ('Kodak', 'KODAK VISION3 500T Color Negative Film 5219', 500, 'Color Negative'),
    ('Kodak', 'KODAK AEROCOLOR IV Negative Film 2460', 125, 'Color Negative'),
    ('Kodak', 'KODAK PROFESSIONAL EKTAR 100 Color Negative Film', 100, 'Color Negative'),
    ('Kodak', 'KODAK PROFESSIONAL PRO IMAGE 100 Color Negative Film', 100, 'Color Negative'),
    ('Kodak', 'KODAK ColorPlus 200 Color Negative Film', 200, 'Color Negative'),
    ('Kodak', 'KODAK GOLD 200 Color Negative Film', 200, 'Color Negative'),
    ('Kodak', 'KODAK UltraMax 400 Color Negative Film', 400, 'Color Negative'),
    ('Kodak', 'KODAK PROFESSIONAL PORTRA 160 Color Negative Film', 160, 'Color Negative'),
    ('Kodak', 'KODAK PROFESSIONAL PORTRA 400 Color Negative Film', 400, 'Color Negative'),
    ('Kodak', 'KODAK PROFESSIONAL PORTRA 800 Color Negative Film', 800, 'Color Negative'),
    ('Kodak', 'KODAK PROFESSIONAL TRI-X 400 Black & White Film', 400, 'B&W'),
    ('Kodak', 'KODAK PROFESSIONAL T-MAX 100 Black & White Film', 100, 'B&W'),
    ('Kodak', 'KODAK PROFESSIONAL T-MAX 400 Black & White Film', 400, 'B&W'),
    ('Kodak', 'KODAK PROFESSIONAL T-MAX P3200 Black & White Film', 3200, 'B&W'),
    ('Kodak', 'KODAK GOLD 800 Color Negative Film', 800, 'Color Negative'),
    ('ORWO', 'ORIGINAL WOLFEN NC400 Color Negative Film', 400, 'Color Negative'),
    ('ORWO', 'ORIGINAL WOLFEN NC500 Color Negative Film', 400, 'Color Negative'),
    ('CineStill', 'CineStill 50D Daylight Fine Grain Color Negative Film', 50, 'Color Negative'),
    ('CineStill', 'CineStill 400Dynamic Versatile Color Negative Film', 400, 'Color Negative'),
    ('CineStill', 'CineStill 800Tungsten High Speed Color Negative Film', 800, 'Color Negative'),
    ('HARMAN', 'HARMAN Phoenix 200 Colour Film', 200, 'Color Negative'),
    ('HARMAN', 'HARMAN Phoenix II 200 Colour Film', 200, 'Color Negative'),
    ('HARMAN', 'HARMAN RED 125 Redscale Film', 125, 'Color Negative'),
    ('HARMAN', 'HARMAN SWITCH Azure 125 Colour Film', 125, 'Color Negative'),
    ('ILFORD', 'ILFORD PAN 100 Black & White Film', 100, 'B&W'),
    ('ILFORD', 'ILFORD PAN 400 Black & White Film', 400, 'B&W'),
    ('ILFORD', 'ILFORD PAN F PLUS 50 Black & White Film', 50, 'B&W'),
    ('ILFORD', 'ILFORD ORTHO PLUS 80 Black & White Film', 80, 'B&W'),
    ('ILFORD', 'ILFORD FP4 PLUS 125 Black & White Film', 125, 'B&W'),
    ('ILFORD', 'ILFORD HP5 PLUS 400 Black & White Film', 400, 'B&W'),
    ('ILFORD', 'ILFORD XP2 SUPER 400 Black & White Film', 400, 'B&W'),
    ('ILFORD', 'ILFORD SFX 200 Black & White Film', 200, 'B&W'),
    ('ILFORD', 'ILFORD DELTA 100 PROFESSIONAL Black & White Film', 100, 'B&W'),
    ('ILFORD', 'ILFORD DELTA 400 PROFESSIONAL Black & White Film', 400, 'B&W'),
    ('ILFORD', 'ILFORD DELTA 3200 PROFESSIONAL Black & White Film', 3200, 'B&W'),
    ('Kentmere', 'Kentmere PAN 100 Black & White Film', 100, 'B&W'),
    ('Kentmere', 'Kentmere PAN 200 Black & White Film', 200, 'B&W'),
    ('Kentmere', 'Kentmere PAN 400 Black & White Film', 400, 'B&W'),
    ('Lomography', 'Lomography Color Negative 100', 100, 'Color Negative'),
    ('Lomography', 'Lomography Color Negative 400', 400, 'Color Negative'),
    ('Lomography', 'Lomography Color Negative 800', 800, 'Color Negative'),
    ('Lomography', 'LomoChrome Metropolis ISO 100-400', 100, 'Color Negative'),
    ('Lomography', 'LomoChrome Purple ISO 100-400', 100, 'Color Negative'),
    ('Lomography', 'LomoChrome Turquoise ISO 100-400', 100, 'Color Negative'),
    ('Lomography', 'LomoChrome Color ''92', 400, 'Color Negative'),
    ('ADOX', 'ADOX CMS 20 II Professional Black & White Film', 20, 'B&W'),
    ('ADOX', 'ADOX SCALA 50 Black & White Reversal Film', 50, 'B&W'),
    ('ADOX', 'ADOX CHS 100 II Black & White Film', 100, 'B&W'),
    ('Rollei', 'Rollei RPX 25 Black & White Negative Film', 25, 'B&W'),
    ('Rollei', 'Rollei RPX 100 Black & White Negative Film', 100, 'B&W'),
    ('Rollei', 'Rollei RPX 400 Black & White Negative Film', 400, 'B&W'),
    ('Rollei', 'Rollei Superpan 200 Black & White Film', 200, 'B&W'),
    ('Rollei', 'Rollei Paul & Reinhold 100 Black & White Film', 100, 'B&W'),
    ('Rollei', 'Rollei Infrared 400 Black & White Film', 400, 'B&W'),
    ('Rollei', 'Rollei Blackbird 100 Black & White Film', 100, 'B&W'),
    ('Rollei', 'Rollei CR200 Color Reversal Film', 200, 'Slide'),
    ('Rollei', 'Rollei Retro 400S Black & White Film', 400, 'B&W'),
    ('Rollei', 'Rollei Retro 80S Black & White Film', 80, 'B&W'),
    ('Lucky', 'Lucky New Generation SHD100 Black & White Photographic Film', 100, 'B&W'),
    ('Lucky', 'Lucky New Generation SHD400 Black & White Photographic Film', 400, 'B&W'),
    ('Lucky', 'Lucky Color 200 Color Negative Film', 200, 'Color Negative'),
    ('Shanghai', 'Shanghai GP3 100 Black & White Film', 100, 'B&W'),
    ('Shanghai', 'Shanghai GP3 400 Black & White Film', 400, 'B&W');

-- Canonicalize the six original sample records in place so their IDs and roll links remain stable.
UPDATE film_stocks SET brand = 'Foma', name = 'FOMAPAN 100 Classic Black & White Negative Film', iso = 100, type = 'B&W'
WHERE lower(brand) = 'foma' AND lower(name) IN ('fomapan 100 classic', 'fomapan 100 classic black & white negative film');
UPDATE film_stocks SET brand = 'Fujifilm', name = 'FUJICOLOR 100 Color Negative Film', iso = 100, type = 'Color Negative'
WHERE lower(brand) = 'fujifilm' AND lower(name) IN ('fujicolor 100', 'fujicolor 100 color negative film');
UPDATE film_stocks SET brand = 'Kodak', name = 'KODAK PROFESSIONAL EKTACHROME E100 Color Reversal Film', iso = 100, type = 'Slide'
WHERE lower(brand) = 'kodak' AND lower(name) IN ('professional ektachrome e100', 'kodak professional ektachrome e100 color reversal film');
UPDATE film_stocks SET brand = 'Kodak', name = 'KODAK PROFESSIONAL PORTRA 160 Color Negative Film', iso = 160, type = 'Color Negative'
WHERE lower(brand) = 'kodak' AND lower(name) IN ('professional portra 160', 'kodak professional portra 160 color negative film');
UPDATE film_stocks SET brand = 'ORWO', name = 'ORIGINAL WOLFEN NC500 Color Negative Film', iso = 400, type = 'Color Negative'
WHERE lower(brand) IN ('wolfen', 'orwo') AND lower(name) IN ('nc500 color negative film', 'original wolfen nc500 color negative film');
UPDATE film_stocks SET brand = 'Kodak', name = 'KODAK GOLD 200 Color Negative Film', iso = 200, type = 'Color Negative'
WHERE lower(brand) = 'kodak' AND lower(name) IN ('gold 200', 'kodak gold 200 color negative film');

-- Refresh canonical records without touching user-maintained status or notes.
UPDATE film_stocks
SET brand = (SELECT source.brand FROM official_films source WHERE lower(source.brand) = lower(film_stocks.brand) AND lower(source.name) = lower(film_stocks.name)),
    name = (SELECT source.name FROM official_films source WHERE lower(source.brand) = lower(film_stocks.brand) AND lower(source.name) = lower(film_stocks.name)),
    iso = (SELECT source.iso FROM official_films source WHERE lower(source.brand) = lower(film_stocks.brand) AND lower(source.name) = lower(film_stocks.name)),
    type = (SELECT source.type FROM official_films source WHERE lower(source.brand) = lower(film_stocks.brand) AND lower(source.name) = lower(film_stocks.name))
WHERE EXISTS (
    SELECT 1 FROM official_films source
    WHERE lower(source.brand) = lower(film_stocks.brand) AND lower(source.name) = lower(film_stocks.name)
);

INSERT INTO film_stocks (brand, name, iso, type, target_status, note)
SELECT source.brand, source.name, source.iso, source.type, 'untested', NULL
FROM official_films source
WHERE NOT EXISTS (
    SELECT 1 FROM film_stocks target
    WHERE lower(target.brand) = lower(source.brand) AND lower(target.name) = lower(source.name)
);

DROP TABLE official_films;
