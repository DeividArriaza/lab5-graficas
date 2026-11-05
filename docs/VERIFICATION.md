# Verificación independiente

Resultados observados el 5 de noviembre de 2026 en Windows, perfil debug:

- `cargo test`: 20 pruebas aprobadas, 0 fallidas.
- `cargo clippy -- -D warnings`: aprobado.
- `cargo build`: aprobado.
- `cargo fmt --check`: aprobado.
- `python tests/verify_render.py --self-test`: 5 pruebas aprobadas, 0 fallidas.
- `python tests/verify_render.py`: 21 comprobaciones aprobadas, 0 fallidas.
- `python tests/verify_render.py --check-existing screenshots`: 8 comprobaciones aprobadas, 0 fallidas.

El verificador ejecuta el binario real en un directorio temporal y comprueba seis PNG RGB8 de 1100 × 760, CRC y descompresión, diversidad de color en el centro, imágenes diferentes, repetibilidad del Sol en tiempo 0, cambio en tiempo 2 y rechazo de argumentos inválidos, incluidos NaN e infinito. La galería de 3300 × 1520 se compara píxel a píxel con las seis imágenes completas, en orden 3 × 2; el autotest rechaza galerías sintéticas duplicadas y de un solo cuerpo. `--check-existing` aplica las comprobaciones de imágenes y galería a las capturas existentes, sin sobrescribirlas.

Se inspeccionó visualmente `screenshots/gallery.png` reducida a 825 × 380: se distinguen los seis cuerpos y el gigante gaseoso muestra anillos y luna separados. También se inspeccionó `screenshots/volcanico.png` a 1100 × 760: una red de canales naranjas y dorados contrasta con el basalto violeta oscuro. La mejora magmática combina basalto, fisuras, núcleos incandescentes y ceniza; la prueba Rust comprueba la presencia de canales cálidos y basalto oscuro. Las imágenes fijas no validan la oclusión durante la animación.

Revisión estática: la CLI valida tiempos finitos (`src/main.rs:90-96`), el renderer utiliza posición local y normales transformadas (`src/renderer.rs:95-104`), y dibuja anillo, esfera y luna como geometrías separadas (`src/renderer.rs:42-69`). Cada cuerpo combina al menos cuatro contribuciones procedurales de color; no se cargan texturas ni materiales externos. Las pruebas de profundidad y posición orbital pasan, pero no sustituyen una inspección visual de la oclusión en movimiento. Observación no bloqueante: el escritor PNG no hace `flush` explícito del `BufWriter` (`src/png.rs:50-52`), por lo que un error de escritura al cerrar podría no propagarse.

Pendiente: los controles interactivos de ventana, rotación, traslación, pausa y captura no se han probado manualmente. El padre intentó abrir la aplicación nativa, pero la captura de `computer_use` agotó el tiempo de espera del proveedor UIA de minifb; cerró el proceso sin verificar los controles. Esto no demuestra un defecto de la aplicación. Su implementación está en `src/main.rs:106-168`; la revisión manual interactiva sigue pendiente.

## QA final independiente — 5 de noviembre de 2026

- `cargo fmt --check`: aprobado; `cargo test`: 25 aprobadas; `cargo clippy --all-targets -- -D warnings`: aprobado; `cargo build --release`: aprobado.
- `python scripts/benchmark.py self-test`: 7 aprobadas; `python tests/verify_render.py --self-test`: 5 aprobadas; integración release: 21/21; `--check-existing screenshots`: 8/8.
- Siete PNG generados con el candidato (seis cuerpos en t=0 y gaseoso en t=2) comparados píxel a píxel con fixtures del binario original: cero píxeles y canales distintos, delta máximo cero. Resultado y hashes: `docs/performance-data/qa-final-images.json`.
- Se inspeccionaron visualmente la galería candidata reducida y las siete capturas individuales: los seis cuerpos conservan detalle reconocible, gaseoso muestra anillos y luna separados en ambos tiempos, sin artefactos evidentes en las imágenes fijas inspeccionadas. Esto no prueba la oclusión durante toda la animación ni controles interactivos.
- Revisión de código: Rayon divide color y profundidad en franjas mutables disjuntas, recorre triángulos en orden dentro de cada franja y completa cada malla antes de la siguiente; no se observó una carrera en esa ruta. La prueba interna compara buffers serial/paralelo para gaseoso t=2. No se observó carga de texturas ni ejecución GPU en el pipeline revisado: shaders y rasterización siguen en CPU. Esto no es una demostración formal de ausencia de carreras o regresiones para todo estado posible.
- Tres rondas de benchmark por cuerpo, procesos seriales y orden alterno: el candidato mejoró la mediana de las medianas en los seis cuerpos; el control adicional Rayon predeterminado versus una hebra también favoreció el predeterminado en esta máquina. Protocolo, dispersión, cifras y limitaciones: `docs/PERFORMANCE.md`.
- SHA-256 de `screenshots/capture.png` antes y después: `6e5b5cc9ad007f01f8c043ef0bd15d7f233ffcab975efcddf0e61068458010c1`. No se incorporó al repositorio.

## Controles básicos — preparación anterior

La sección anterior corresponde al QA de rendimiento anterior, no verifica los controles nuevos. El verificador de controles se preparó para pruebas exclusivamente headless; las pruebas reales de controles de ventana y una inspección manual interactiva siguen pendientes. El intento previo de captura UIA agotó el tiempo de espera, por lo que no se afirma que haya existido inspección interactiva satisfactoria.

Preparación: el autotest del verificador pasó 4/4. Una prueba RED enfocada con el ejecutable baseline anterior confirmó que rechazaba `--help`; eso no evalúa la implementación nueva. Tras compilar el candidato, `python tests/verify_controls.py` funciona desde un clon sin caché Hermes: compara los seis valores predeterminados en t=0 con `screenshots/` y compara valores implícitos y explícitos también para gaseoso t=2. Sin fixtures no comprueba gaseoso t=2 contra un original, y lo informa explícitamente. Para QA estricto usar `python tests/verify_controls.py target/release/lab5-graficas.exe --fixtures C:/Users/david/AppData/Local/hermes/cache/scratch/lab5-baseline-fixtures`: requiere los siete PNG originales y falla si falta alguno. Las salidas temporales se generan fuera del repositorio; para ubicarlas en scratch, definir `QA_SCRATCH=C:/Users/david/AppData/Local/hermes/cache/scratch` antes de ejecutar.

## QA controles — 5 de noviembre de 2026: informe histórico

- Esta entrada conserva el estado observado antes de corregir la ayuda de CLI. El bloqueo descrito aquí ya no es el estado actual.
- En esa ejecución, `cargo test` pasó 36/36; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `cargo build --release` aprobaron. El aviso transitorio de clippy en `src/hud.rs` fue corregido por su responsable y el reintento pasó.
- En esa ejecución, los autotests Python pasaron: `verify_render.py` 5/5, `scripts/benchmark.py` 7/7 y `verify_controls.py` 4/4. El verificador gráfico release pasó 21/21 y las capturas existentes 8/8.
- Antes de ampliar el gate de ayuda, `verify_controls.py` pasó 7/7 tanto en modo portable como con `--fixtures`; este último comprobó los siete PNG originales a nivel RGB, sin diferencias. Brillo y detalle modificaron píxeles manteniendo dimensiones; la ayuda opcional modifica la exportación individual y la salida predeterminada permanece limpia.
- **Bloqueo histórico:** en ese momento `--help` omitía opciones y el gate de ayuda fallaba por `--time` ausente. Al retomar la tarea, `src/main.rs` ya estaba corregido; no se modificó código durante este cierre documental.
- Revisión de cableado: `Controls` modifica `renderer.shader_settings`; R no modifica pausa ni cámara; la captura S ocurre después de render y antes de dibujar HUD. La ayuda empieza visible. Se inspeccionó `screenshots/controles.png`: panel legible; la prueba headless no sustituye teclado físico ni una sesión interactiva.
- `screenshots/capture.png`: SHA-256 al inicio y al final de esta tarea `275552c032ba54dca6995fa45f987cd45bf2cf16422f13904df304964b7c74d4`; no se modificó ni se incluyó en stage. Sin benchmarks nuevos ni modificación de historial de rendimiento.

## QA de cierre de release — 5 de noviembre de 2026

Resultados ejecutados y reportados por el orquestador sobre el binario `target/release/lab5-graficas.exe`; no fueron ejecutados por quien editó esta documentación:

- `cargo test --locked`: 36/36; `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` y `cargo build --release --locked`: aprobados.
- Autotests: `verify_controls.py` 4/4, `verify_render.py` 5/5 y `scripts/benchmark.py` 7/7.
- Verificación controls release con fixtures originales: 7/7, incluidos los siete PNG y `help_exits_without_window`. La ejecución portable también pasó 7/7.
- Verificación render release: 21/21; `--check-existing screenshots`: 8/8. Las capturas existentes se comprobaron sin sobrescribirlas.
- La salida actual de `target/release/lab5-graficas.exe --help` se comprobó directamente durante este cierre y enumera las opciones de CLI, incluidas `--time`, `--render-all`, `--headless`, `--output`, `--benchmark`, `--frames` y `--warmup`.
- Revisión de shaders y controles reportada por Terra: sin defectos; no requirió edición de código. No se ejecutaron benchmarks nuevos ni se afirma una prueba de teclado físico.
- SHA-256 actual registrado para `screenshots/capture.png`: `275552c032ba54dca6995fa45f987cd45bf2cf16422f13904df304964b7c74d4` (confirmado directamente durante este cierre). Se conserva la captura.

Estado actual: QA headless de release aprobado. La comprobación interactiva con teclado físico sigue pendiente.

## Cambio de teclas de brillo — 5 de noviembre de 2026

Solicitud del usuario: `Z` baja y `X` sube el brillo en pasos de 0.1; se conservan los límites 0.25–2 y `R` restablece brillo y detalle. `Q/E` conserva el ajuste de detalle. Las pruebas enfocadas se actualizaron primero y dieron RED con el binding previo (`Key::X` sin manejar); tras cambiar los controles y la lista de teclas de ventana, pasaron controles 3/3, HUD 1/1 y ayuda CLI 1/1. También se comprobó que las teclas anteriores de más/menos no alteran brillo. Después de mover el helper HUD antes del módulo de pruebas, `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` y `cargo build --release --locked` pasaron. El orquestador reportó también `cargo test --locked` 36/36 y los gates finales verdes. Se exportó `screenshots/controles.png` con el binario release usando `--headless --body gaseoso --time 0 --help-overlay`; el hash de `screenshots/capture.png` permaneció `6c13c0ea514ceff8286cdde18d7e6c60cfdb0ea955d4320ca9b3938093959572`.
