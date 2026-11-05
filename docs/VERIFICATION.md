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