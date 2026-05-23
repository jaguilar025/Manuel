# Manuel

<img src="logo.png" alt="Manuel logo" width="120" />

**Manuel** es una aplicación de escritorio ligera para **Ubuntu Linux** que mapea
entradas de teclado o dispositivos HID externos (botones USB, gamepads, mini
keypads, etc.) hacia acciones de salida: escribir texto, pulsar teclas,
ejecutar combinaciones, macros o comandos shell.

Pensada para flujos personales donde necesitas convertir un botón físico o un
atajo de teclado en una acción concreta sin instalar suites pesadas tipo
AntiMicroX. Corre en background con icono en la bandeja del sistema y consume
muy poca RAM.

---

## Características

- Mapping de **botones HID** a salidas: texto, tecla, combo, macro o shell.
- Mapping de **combinaciones de teclado** globales (Ctrl+Alt+P, Ctrl+Shift+F13, etc.).
- **Input Recorder**: detecta el botón / combinación que pulsas y lo guarda solo.
- **Identificación estable por vendor+product ID** — los mappings sobreviven a
  reboots, replugs y cambios de sesión.
- **Macros** con `type`, `press`, `combo`, `delay` y `shell`.
- **Soporte Unicode completo** (`ñ`, acentos, símbolos) vía layout XKB + dead keys
  automáticos. Sin parpadeo de pantalla.
- **Icono en la bandeja** (tray): cerrar la ventana sólo la oculta; la app
  sigue corriendo.
- **Autostart** opcional al iniciar sesión.
- Configuración en JSON plano en `~/.config/manuel/config.json`.

---

## Instalación (usuarios finales)

### 1. Dependencias del sistema

```bash
sudo apt install -y \
  ydotool \
  wl-clipboard \
  libayatana-appindicator3-1 \
  libwebkit2gtk-4.1-0
```

### 2. Permitir acceso al dispositivo de inyección

Manuel necesita escribir en `/dev/uinput` (a través de `ydotoold`) para
sintetizar teclas. Hay que dar acceso a tu usuario una sola vez:

```bash
# Regla udev: /dev/uinput accesible por el grupo "input"
echo 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"' \
  | sudo tee /etc/udev/rules.d/80-uinput.rules
sudo udevadm control --reload-rules
sudo udevadm trigger /dev/uinput

# Añade tu usuario al grupo input
sudo usermod -aG input $USER
```

**Cierra sesión y vuelve a entrar** para que el cambio de grupo tome efecto.

Verifica:

```bash
ls -la /dev/uinput   # debe ser:  crw-rw---- 1 root input
groups | grep input  # debe aparecer "input"
```

### 3. Instalar Manuel

Descarga el `.deb` desde la sección de releases y:

```bash
sudo apt install ./Manuel_0.1.0_amd64.deb
```

O si prefieres el AppImage portable:

```bash
chmod +x Manuel_0.1.0_amd64.AppImage
./Manuel_0.1.0_amd64.AppImage
```

### 4. Lanzar

Busca **Manuel** en el menú de aplicaciones, o desde terminal:

```bash
manuel
```

Aparecerá un icono en la bandeja superior. La primera vez no tendrás mappings;
crea uno desde la pestaña **Mappings**.

---

## Uso

### Crear un mapping

1. Abre la pestaña **Mappings** → **+ New mapping**.
2. Elige el **input**:
   - `Keyboard combo` para combinaciones (Ctrl+Alt+P, F13, etc.).
   - `HID button` para botones USB externos.
3. Click en **Detect** y pulsa el botón / combo. Se guardará automáticamente.
4. Elige el **output**:
   - `Type text` — escribe texto literal (soporta Unicode).
   - `Press key` — una tecla (`Enter`, `F13`, `a`, etc.).
   - `Press combo` — combinación tipo `Ctrl+Shift+P`.
   - `Run macro` — secuencia JSON, ejemplo:
     ```json
     [
       {"op":"type","text":"Hola"},
       {"op":"delay","ms":100},
       {"op":"press","key":"Enter"}
     ]
     ```
   - `Shell command` — ejecuta un comando del sistema.
5. Usa el toggle verde (a la derecha) para activar/desactivar el mapping sin borrarlo.

### Tray

- **Click** en el icono → muestra/oculta la ventana.
- **Click derecho** → menú: Show/Hide, Exit.
- **Cerrar la ventana** (X) no cierra la app; sigue corriendo en background.

### Settings

- **Start on system boot** — crea/borra `~/.config/autostart/manuel.desktop`.
- **Start minimized** — arranca sin abrir la ventana.
- **Run in tray** — controla si cerrar la ventana oculta o sale.
- **Enable notifications** — reservado.

---

## Resolución de problemas

| Síntoma | Causa probable | Fix |
|---|---|---|
| Los outputs no escriben nada | `ydotoold` no arrancó | Verifica permisos de `/dev/uinput` y grupo `input`. Reinstala la regla udev. |
| `ñ` o acentos no aparecen | Layout de teclado sin esos caracteres | Cambia layout a *English (US, intl., with dead keys)* o español en GNOME Settings → Keyboard. |
| Un mapping deja de funcionar tras reboot | Mapping creado con versión vieja sin vendor/product ID | Borra el mapping y vuelve a grabarlo con **Detect**. |
| No aparece icono en la bandeja | Falta `libayatana-appindicator3-1` o extensión de tray en GNOME 45+ | `sudo apt install gnome-shell-extension-appindicator`. |
| La app no detecta tu dispositivo | Tu usuario no está en el grupo `input` | `sudo usermod -aG input $USER` y relogin. |

---

## Compilar desde código fuente

Requisitos: Rust ≥ 1.77, Node ≥ 18.

```bash
sudo apt install -y build-essential pkg-config libssl-dev \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libudev-dev

git clone https://github.com/jaguilar025/Manuel.git manuel
cd manuel
npm install

# Desarrollo
npm run tauri:dev

# Empaquetar
npm run tauri:build
# Artefactos en: src-tauri/target/release/bundle/{deb,appimage}/
```

---

## Arquitectura

```
Frontend (Vue 3 + Vite + Pinia + Tailwind)
        │  Tauri IPC (JSON)
Backend (Rust):
        ├─ engine: un hilo por device, lectura evdev, routing
        ├─ recorder: captura el siguiente input cuando está "armed"
        ├─ keyboard / macro_runner: emisión vía ydotool (+ XKB fallback)
        ├─ textmap: mapa char→keycode según layout XKB activo
        ├─ ydotoold: spawn/stop del daemon como hijo de la app
        ├─ storage: persistencia atómica del config.json
        ├─ autostart: archivo .desktop en ~/.config/autostart/
        └─ tray/window: integración con la bandeja del sistema
```

- **Captura**: `evdev` directo sobre `/dev/input/event*` (funciona en X11 y Wayland, distingue por dispositivo).
- **Emisión**: `ydotool` + `ydotoold` (escribe en `/dev/uinput` a nivel kernel; funciona en cualquier compositor).
- **Unicode**: librería `xkbcommon` para resolver `char → (keycode, mods)` según el layout activo del sistema; dead keys automáticos vía descomposición Unicode NFD.

---

## Licencia

Uso personal. Sin garantías.

---

by **jaguilar025** | 2026
