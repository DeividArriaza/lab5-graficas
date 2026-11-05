# Benchmark de renderizado CPU

`--benchmark` ejecuta el renderer sin ventana ni PNG y emite JSON a stdout. Sin `--body` mide los seis cuerpos; con `--body gaseoso` mide solo ese cuerpo. `--frames` debe ser positivo y `--warmup` no negativo; por defecto son 12 y 2. `--time` indica el tiempo inicial. No combinar con `--output` ni `--render-all`.

```sh
cargo build --release
target/release/lab5-graficas.exe --benchmark --body gaseoso --time 0 --warmup 2 --frames 12
```

La inicialización, mallas, salida y estadísticas quedan fuera de la medición. Cada muestra usa `Instant` alrededor de `renderer.render`, incluido el clear que hace el propio render. Tras cada render, fuera del intervalo, `black_box` consume una referencia al framebuffer. Para cada cuerpo la secuencia es `time + i / 60`, con i desde 0 incluyendo warmup; los frames de calentamiento se descartan. Repetir exactamente tiempo, cuerpos, frames y warmup antes y después de cambios. Cada muestra es ms/frame; mediana convencional, p95 por rango más cercano (ceil(0.95*N)), mínimo, máximo y FPS equivalente 1000/mediana. Son tiempos de render CPU, no FPS medidos de ventana ni una promesa de rendimiento.

## Baseline — 5 de noviembre de 2026

Código base HEAD `3d0b33a541fd7120e6a0ca6f35a2282885d32f90`, con la instrumentación de benchmark en `src/main.rs`, sin cambios al renderer ni shaders. Windows build 26200.9711; Intel Core i9-13980HX, 24 núcleos físicos y 32 lógicos; rustc 1.99.1, perfil release, framebuffer 1100 × 760. Procesos por cuerpo ejecutados serialmente, sin compilaciones concurrentes; carga y frecuencia del sistema no controladas. Resultados orientativos de una sola corrida, 12 muestras: el p95 es el máximo para N=12.

| Cuerpo | Mediana ms | p95 ms | FPS equivalente |
|---|---:|---:|---:|
| sol | 363.400 | 696.100 | 2.75 |
| rocoso | 226.148 | 380.867 | 4.42 |
| gaseoso | 182.983 | 186.726 | 5.46 |
| oceanico | 376.716 | 685.899 | 2.65 |
| volcanico | 217.090 | 332.321 | 4.61 |
| helado | 269.628 | 500.482 | 3.71 |

Artefactos fuera del repositorio: `C:/Users/david/AppData/Local/hermes/cache/scratch/lab5-fps-baseline.exe`, `lab5-baseline.json`, un JSON por cuerpo `lab5-baseline-<cuerpo>.json` y `lab5-baseline-fixtures/`. Los siete PNG provienen del ejecutable original `lab5-original.exe` con `--headless --body <cuerpo> --time <0|2> --output <ruta>`: seis cuerpos en t=0 y gaseoso adicional en t=2. No se utilizó `screenshots/capture.png`.

Comando exacto de medición (Git Bash, desde cualquier directorio):

```sh
export SCRATCH='C:/Users/david/AppData/Local/hermes/cache/scratch'
for body in sol rocoso gaseoso oceanico volcanico helado; do "$SCRATCH/lab5-fps-baseline.exe" --benchmark --frames 12 --warmup 2 --body "$body" --time 0 > "$SCRATCH/lab5-baseline-$body.json" || exit; done
```

Las muestras completas están en JSON; los seis archivos se combinaron sin modificar muestras ni estadísticas en `lab5-baseline.json`. Mantener este ejecutable y estas imágenes para comparación posterior. Los archivos en scratch son temporales y pueden ser eliminados por la limpieza automática del entorno.

Copia durable y sin modificación de los siete JSON de baseline: `docs/performance-data/` (un consolidado y seis salidas crudas individuales). El SHA-256 del ejecutable de baseline instrumentado es `12c5b701b07c02c824206af4e794631579c1331edaf83f74e82a889f966c77ae`; el ejecutable y los PNG siguen solamente en scratch. La copia durable no sustituye al binario necesario para mediciones pareadas. Los picos de la baseline inicial se conservan en las muestras; no se descartan.

## Protocolo de QA posterior a la integración

Para futuras repeticiones, coordinar con los implementadores y ejecutar sin builds concurrentes. Compilar y medir sin otros builds ni cargas importantes; no comparar con un binario mientras se recompila. El siguiente protocolo conserva las siete referencias originales y escribe únicamente fuera del repositorio. Use rutas de salida nuevas: el script rechaza directorios y JSON existentes.

```sh
export SCRATCH='C:/Users/david/AppData/Local/hermes/cache/scratch'
export QA_SCRATCH="$SCRATCH"
python scripts/benchmark.py self-test
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
python tests/verify_render.py --self-test
python tests/verify_render.py target/release/lab5-graficas.exe
python scripts/benchmark.py capture --exe target/release/lab5-graficas.exe --output-dir "$SCRATCH/lab5-optimized-fixtures"
python scripts/benchmark.py compare --reference "$SCRATCH/lab5-baseline-fixtures" --candidate "$SCRATCH/lab5-optimized-fixtures" --output "$SCRATCH/lab5-qa-image-comparison.json"
python scripts/benchmark.py run --baseline "$SCRATCH/lab5-fps-baseline.exe" --candidate target/release/lab5-graficas.exe --output-dir "$SCRATCH/lab5-qa-performance" --frames 12 --warmup 2 --time 0 --trials 3
```

`capture` usa los mismos seis cuerpos en t=0 y gaseoso en t=2. `compare` decodifica ambos PNG con `tests/verify_render.py` y compara los tres canales de cada píxel: registra número de píxeles y canales distintos, delta máximo absoluto (0–255), dimensiones y SHA-256 de cada archivo. Cualquier diferencia devuelve código 1 y requiere revisión: la política por defecto exige igualdad exacta, sin tolerancia para ruido ni pérdida de detalle; una diferencia pequeña no se acepta automáticamente. Errores de lectura, dimensiones o formato devuelven 2. Inspeccionar manualmente las siete imágenes optimizadas, especialmente anillos, luna, detalle y animación, y documentar la inspección real.

`run` lanza procesos seriales por cuerpo y versión; las tres rondas alternan baseline/candidato, candidato/baseline, baseline/candidato. Cada proceso usa exactamente los mismos `frames`, `warmup`, `time` y cuerpo; reutiliza el mismo ejecutable de cada versión. Guarda el stdout JSON original de cada ejecución (36 archivos), hashes y parámetros en `metadata.json` y un `summary.json` con la mediana de las medianas por cuerpo, speedup baseline/candidato y FPS equivalentes 1000/ms. Incluye las tres medianas, su rango, los p95 individuales y el rango de todas las muestras, sin seleccionar la ronda más favorable. Valida muestras y estadísticas y verifica que los ejecutables no cambiaron durante el ensayo. No son FPS de ventana. La primera y tercera ronda favorecen el mismo orden; los resultados siguen sujetos a ruido térmico, frecuencia y procesos externos. Repetir con directorio nuevo si hace falta y conservar todas las rondas. El comando no compila. Tras QA, copiar los JSON de resultados reales a `docs/performance-data/` sin incorporar binarios ni PNG.

## Resultado QA final — 5 de noviembre de 2026

Intel i9-13980HX (24 núcleos físicos, 32 lógicos), Windows 11 build 26200.9711, rustc 1.99.1, release 1100 × 760. `RAYON_NUM_THREADS` no estaba definido. Tres rondas seriales de procesos, 2 warmup y 12 muestras por ronda, tiempo inicial 0; sin builds concurrentes. El candidato usa Rayon y shaders optimizados; el baseline usa el renderer original instrumentado. Las columnas p95 son la mediana de los tres p95 por ejecución, no un percentil combinado. Con 12 muestras cada p95 es el máximo de esa ejecución. Los rangos son mínimos y máximos de las tres medianas, sin descartar picos.

| Cuerpo | Baseline ms | Candidato ms | p95 base/candidato ms | FPS base/candidato | Speedup | Rango medianas base ms | Rango medianas candidato ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| sol | 366.97 | 44.10 | 668.08 / 44.82 | 2.73 / 22.68 | 8.32× | 335.92–384.96 | 43.95–45.49 |
| rocoso | 218.15 | 32.50 | 301.44 / 33.62 | 4.58 / 30.77 | 6.71× | 217.71–219.82 | 32.00–32.78 |
| gaseoso | 186.54 | 32.61 | 224.03 / 35.00 | 5.36 / 30.67 | 5.72× | 185.59–192.70 | 30.01–32.91 |
| oceanico | 472.36 | 44.50 | 671.68 / 47.98 | 2.12 / 22.47 | 10.62× | 377.91–508.13 | 44.36–45.82 |
| volcanico | 216.13 | 27.71 | 277.93 / 33.13 | 4.63 / 36.08 | 7.80× | 215.40–217.16 | 27.63–28.34 |
| helado | 260.46 | 32.43 | 504.38 / 36.32 | 3.84 / 30.84 | 8.03× | 257.56–307.19 | 32.11–33.62 |

Los datos completos, rangos de muestras, 36 JSON crudos y metadatos están en `docs/performance-data/qa-final-performance/`. Las siete comparaciones exactas están en `docs/performance-data/qa-final-images.json`: cero píxeles y canales distintos y delta máximo cero. El mismo ejecutable candidato también se ensayó en otras tres rondas alternadas con Rayon predeterminado frente a `RAYON_NUM_THREADS=1`: razones de mediana de 5.03× a 8.00× a favor del valor predeterminado en esta máquina. Datos en `docs/performance-data/qa-thread-probe/`. Una hebra de Rayon sigue usando el rasterizado por bandas, no equivale al modo serial interno; esta medición no aísla perfectamente todos los efectos. Las cifras no prometen 60 FPS ni representan presentación en ventana.

El parche `docs/performance-data/baseline-instrumentation.patch` se generó con `--unified=0` contra `3d0b33a` solo para `src/main.rs`. Para reconstruir en un checkout separado y limpio de ese commit, aplicar `git apply --unidiff-zero <ruta-absoluta-al-parche>` y ejecutar `cargo build --release`. Se comprobó la aplicación sobre el archivo original en scratch y coincidencia del contenido normalizando finales de línea; un binario reconstruido podría no tener el mismo hash por diferencias del entorno/toolchain.
