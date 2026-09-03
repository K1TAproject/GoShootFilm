CREATE TEMP TABLE builtin_film_keys (
    brand TEXT NOT NULL,
    name TEXT NOT NULL
);

INSERT INTO builtin_film_keys (brand, name) VALUES
    ('Foma', 'FOMAPAN 100 Classic Black & White Negative Film'),
    ('Foma', 'FOMAPAN 200 Creative Black & White Negative Film'),
    ('Foma', 'FOMAPAN 400 Action Black & White Negative Film'),
    ('Foma', 'FOMAPAN R 100 Black & White Reversal Film'),
    ('Foma', 'RETROPAN 320 soft Black & White Negative Film'),
    ('Foma', 'FOMA Ortho 400 Black & White Negative Film'),
    ('Fujifilm', 'FUJICOLOR 100 Color Negative Film'),
    ('Fujifilm', 'FUJIFILM 200 Color Negative Film'),
    ('Fujifilm', 'FUJIFILM 400 Color Negative Film'),
    ('Fujifilm', 'FUJICOLOR PRO 400H Professional Color Negative Film'),
    ('Fujifilm', 'FUJICOLOR SUPERIA X-TRA 400 Color Negative Film'),
    ('Fujifilm', 'FUJICOLOR SUPERIA PREMIUM 400 Color Negative Film'),
    ('Fujifilm', 'NEOPAN 100 ACROS II Black & White Negative Film'),
    ('Fujifilm', 'FUJICHROME PROVIA 100F Professional Color Reversal Film'),
    ('Fujifilm', 'FUJICHROME Velvia 50 Professional Color Reversal Film'),
    ('Fujifilm', 'FUJICHROME Velvia 100 Professional Color Reversal Film'),
    ('Kodak', 'KODAK PROFESSIONAL EKTACHROME E100 Color Reversal Film'),
    ('Kodak', 'EASTMAN DOUBLE-X Black & White Negative Film 5222'),
    ('Kodak', 'KODAK EKTACHROME 100D Color Reversal Film 5294'),
    ('Kodak', 'KODAK VISION3 50D Color Negative Film 5203'),
    ('Kodak', 'KODAK VISION3 250D Color Negative Film 5207'),
    ('Kodak', 'KODAK VISION3 200T Color Negative Film 5213'),
    ('Kodak', 'KODAK VISION3 500T Color Negative Film 5219'),
    ('Kodak', 'KODAK AEROCOLOR IV Negative Film 2460'),
    ('Kodak', 'KODAK PROFESSIONAL EKTAR 100 Color Negative Film'),
    ('Kodak', 'KODAK PROFESSIONAL PRO IMAGE 100 Color Negative Film'),
    ('Kodak', 'KODAK ColorPlus 200 Color Negative Film'),
    ('Kodak', 'KODAK GOLD 200 Color Negative Film'),
    ('Kodak', 'KODAK UltraMax 400 Color Negative Film'),
    ('Kodak', 'KODAK PROFESSIONAL PORTRA 160 Color Negative Film'),
    ('Kodak', 'KODAK PROFESSIONAL PORTRA 400 Color Negative Film'),
    ('Kodak', 'KODAK PROFESSIONAL PORTRA 800 Color Negative Film'),
    ('Kodak', 'KODAK PROFESSIONAL TRI-X 400 Black & White Film'),
    ('Kodak', 'KODAK PROFESSIONAL T-MAX 100 Black & White Film'),
    ('Kodak', 'KODAK PROFESSIONAL T-MAX 400 Black & White Film'),
    ('Kodak', 'KODAK PROFESSIONAL T-MAX P3200 Black & White Film'),
    ('Kodak', 'KODAK GOLD 800 Color Negative Film'),
    ('ORWO', 'ORIGINAL WOLFEN NC400 Color Negative Film'),
    ('ORWO', 'ORIGINAL WOLFEN NC500 Color Negative Film'),
    ('CineStill', 'CineStill 50D Daylight Fine Grain Color Negative Film'),
    ('CineStill', 'CineStill 400Dynamic Versatile Color Negative Film'),
    ('CineStill', 'CineStill 800Tungsten High Speed Color Negative Film'),
    ('HARMAN', 'HARMAN Phoenix 200 Colour Film'),
    ('HARMAN', 'HARMAN Phoenix II 200 Colour Film'),
    ('HARMAN', 'HARMAN RED 125 Redscale Film'),
    ('HARMAN', 'HARMAN SWITCH Azure 125 Colour Film'),
    ('ILFORD', 'ILFORD PAN 100 Black & White Film'),
    ('ILFORD', 'ILFORD PAN 400 Black & White Film'),
    ('ILFORD', 'ILFORD PAN F PLUS 50 Black & White Film'),
    ('ILFORD', 'ILFORD ORTHO PLUS 80 Black & White Film'),
    ('ILFORD', 'ILFORD FP4 PLUS 125 Black & White Film'),
    ('ILFORD', 'ILFORD HP5 PLUS 400 Black & White Film'),
    ('ILFORD', 'ILFORD XP2 SUPER 400 Black & White Film'),
    ('ILFORD', 'ILFORD SFX 200 Black & White Film'),
    ('ILFORD', 'ILFORD DELTA 100 PROFESSIONAL Black & White Film'),
    ('ILFORD', 'ILFORD DELTA 400 PROFESSIONAL Black & White Film'),
    ('ILFORD', 'ILFORD DELTA 3200 PROFESSIONAL Black & White Film'),
    ('Kentmere', 'Kentmere PAN 100 Black & White Film'),
    ('Kentmere', 'Kentmere PAN 200 Black & White Film'),
    ('Kentmere', 'Kentmere PAN 400 Black & White Film'),
    ('Lomography', 'Lomography Color Negative 100'),
    ('Lomography', 'Lomography Color Negative 400'),
    ('Lomography', 'Lomography Color Negative 800'),
    ('Lomography', 'LomoChrome Metropolis ISO 100-400'),
    ('Lomography', 'LomoChrome Purple ISO 100-400'),
    ('Lomography', 'LomoChrome Turquoise ISO 100-400'),
    ('Lomography', 'LomoChrome Color ''92'),
    ('ADOX', 'ADOX CMS 20 II Professional Black & White Film'),
    ('ADOX', 'ADOX SCALA 50 Black & White Reversal Film'),
    ('ADOX', 'ADOX CHS 100 II Black & White Film'),
    ('Rollei', 'Rollei RPX 25 Black & White Negative Film'),
    ('Rollei', 'Rollei RPX 100 Black & White Negative Film'),
    ('Rollei', 'Rollei RPX 400 Black & White Negative Film'),
    ('Rollei', 'Rollei Superpan 200 Black & White Film'),
    ('Rollei', 'Rollei Paul & Reinhold 100 Black & White Film'),
    ('Rollei', 'Rollei Infrared 400 Black & White Film'),
    ('Rollei', 'Rollei Blackbird 100 Black & White Film'),
    ('Rollei', 'Rollei CR200 Color Reversal Film'),
    ('Rollei', 'Rollei Retro 400S Black & White Film'),
    ('Rollei', 'Rollei Retro 80S Black & White Film'),
    ('Lucky', 'Lucky New Generation SHD100 Black & White Photographic Film'),
    ('Lucky', 'Lucky New Generation SHD400 Black & White Photographic Film'),
    ('Lucky', 'Lucky Color 200 Color Negative Film'),
    ('Shanghai', 'Shanghai GP3 100 Black & White Film'),
    ('Shanghai', 'Shanghai GP3 400 Black & White Film');

-- Records inserted by the official catalog migration still carry its old default.
-- Exclude the six original seed rows here so a user-changed value on those rows is preserved.
UPDATE film_stocks
SET target_status = 'unshot'
WHERE target_status = 'untested'
  AND NOT (
      id BETWEEN 1 AND 6
      AND created_at IN ('2026-08-30 08:30:09', '2026-08-30 08:36:28', '2026-08-31 08:50:00')
  )
  AND EXISTS (
      SELECT 1
      FROM builtin_film_keys builtin
      WHERE lower(builtin.brand) = lower(film_stocks.brand)
        AND lower(builtin.name) = lower(film_stocks.name)
  );

-- Correct only untouched historical defaults from the original six seed records.
UPDATE film_stocks SET target_status = 'unshot'
WHERE id = 1 AND brand = 'Foma' AND name = 'FOMAPAN 100 Classic Black & White Negative Film'
  AND created_at = '2026-08-30 08:30:09' AND target_status = 'shot';
UPDATE film_stocks SET target_status = 'unshot'
WHERE id = 3 AND brand = 'Kodak' AND name = 'KODAK PROFESSIONAL EKTACHROME E100 Color Reversal Film'
  AND created_at = '2026-08-30 08:36:28' AND target_status = 'shot';
UPDATE film_stocks SET target_status = 'unshot'
WHERE id = 4 AND brand = 'Kodak' AND name = 'KODAK PROFESSIONAL PORTRA 160 Color Negative Film'
  AND created_at = '2026-08-30 08:36:28' AND target_status = 'shot';
UPDATE film_stocks SET target_status = 'unshot'
WHERE id = 6 AND brand = 'Kodak' AND name = 'KODAK GOLD 200 Color Negative Film'
  AND created_at = '2026-08-31 08:50:00' AND target_status = 'shot';

DROP TABLE builtin_film_keys;
