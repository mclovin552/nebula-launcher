# Nebula Launcher

Lanzador de Minecraft Java de código abierto para Linux y Windows, hecho con Tauri 2 (Rust) y una interfaz web nativa sin dependencias de Node.

[![Licencia](https://img.shields.io/badge/licencia-MIT-blue.svg)](LICENSE)

## Funciones

- **Gestión de instancias** completas: vanilla, Fabric, Quilt, Forge y NeoForge, con grupos, colores, memoria y Java por instancia.
- **Instalación de contenido desde Modrinth**: mods, modpacks (`.mrpack`), texturas y shaders, con resolución automática de dependencias.
- **⚡ Optimizar**: botón que instala de golpe los mods de rendimiento compatibles (Sodium, Lithium, Embeddium, ModernFix, etc.).
- **Verificar y reparar**: comprobación SHA-1 de todos los archivos del juego y re-descarga de los dañados.
- **Estado de servidores**: ping real (MOTD, jugadores, latencia) desde la pestaña de Servidores.
- **Entrada rápida a servidores** al iniciar el juego (quickPlay).
- **Skins y capas**: armario de skins con vista previa, cambio por la API oficial para cuentas Microsoft y skins/capas locales con CustomSkinLoader para cuentas sin conexión.
- **Importación**: instancias de Prism Launcher/MultiMC (`.zip`) y modpacks de Modrinth (`.mrpack`).
- **Exportación** de instancias como modpack `.mrpack`.
- **Iconos personalizados** por instancia.
- **Discord Rich Presence** opcional.
- **Cuentas offline** y Microsoft (login OAuth por localhost; sin pedir Client ID).
- **Bandeja del sistema**: la ventana se minimiza a la bandeja al cerrarla (opcional).
- **Copias de seguridad de mundos**, consola integrada con detección de errores comunes y actualizador de launcher opcional.

## Instalación

### Linux

Descarga el `.AppImage`, `.deb` o `.rpm` desde la sección **Releases** y ejecuta:

```bash
chmod +x nebula-launcher_*.AppImage
./nebula-launcher_*.AppImage
```

### Windows

Descarga el instalador `.msi` o `.exe` (NSIS) desde **Releases** y ejecútalo. Windows puede mostrar un aviso de "editor desconocido": pulsa "Más información → Ejecutar de todas formas" (es un binario sin firma, algo normal en proyectos de código abierto sin certificado).

## Compilar desde el código

Requisitos: Rust 1.70+.

Linux (Fedora): `sudo dnf install webkit2gtk4.1-devel openssl-devel gcc librsvg2-devel libxdo-devel`

Linux (Ubuntu/Debian): `sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf build-essential curl`

```bash
git clone https://github.com/TU_USUARIO/nebula-launcher
cd nebula-launcher/src-tauri
cargo run --release
```

Binario suelto: `target/release/nebula-launcher`.

## Generar instaladores

En cada plataforma se generan sus propios paquetes:

- Windows: `.msi` (WiX) y `.exe` (NSIS)
- Linux: `.AppImage`, `.deb`, `.rpm`
- macOS: `.app` / `.dmg` (agrega igual su job al workflow si lo quieres)

Este repositorio incluye un workflow de **GitHub Actions** (`.github/workflows/release.yml`): al enviar una etiqueta `v*`, GitHub construye automáticamente los instaladores de Windows y Linux y crea un borrador de Release con todos los archivos. Solo tienes que hacer:

```bash
git tag v1.0.0
git push origin v1.0.0
```

## Configuración opcional

- `NEBULA_MS_CLIENT_ID`: Client ID de Azure propio para el login Microsoft (redirect `http://127.0.0.1:<puerto>/callback`). El valor interno ya viene configurado para el flujo localhost.
- `NEBULA_DISCORD_APP_ID`: Application ID de Discord para habilitar el Rich Presence (también se puede pegar en Ajustes).
- `update_repo`: repositorio `usuario/repo` de GitHub en Ajustes para activar el chequeo de actualizaciones del launcher.

## Estructura del proyecto

```
ui/                 Interfaz (vanilla JS/CSS, sin bundler)
src-tauri/src/      Backend Rust (Tauri 2)
  instances.rs      Instancias y contenido
  modrinth.rs       Búsqueda/instalación Modrinth y optimización
  launch.rs         Proceso del juego, logs, RPC
  accounts.rs       OAuth Microsoft (localhost) + cuentas offline
  wardrobe.rs       Skins y capas
  serverping.rs     Ping SLP a servidores
  discord.rs        Discord Rich Presence vía IPC local
```

## Licencia

MIT — ver [LICENSE](LICENSE).
