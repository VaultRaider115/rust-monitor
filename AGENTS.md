# AGENTS.md — Agent Monitor

Contexto del repositorio para agentes de código. Léelo antes de tocar nada.

## Qué es

Monitor de sistema para Linux escrito en C++20. Levanta un servidor Crow en el
puerto `8080` con un WebSocket en `/ws`. Al abrir la conexión envía el hostname;
cuando el cliente manda `metrics`, responde un JSON con CPU, RAM, disco y red.

**Restricción dura:** el proyecto debe ejecutarse como proceso nativo en Linux.
No uses Docker, Podman, Kubernetes, Compose, Helm, servicios cloud ni ninguna
plataforma de orquestación o despliegue administrado. La compilación y la
ejecución son locales, con el toolchain del sistema.

## Stack

- C++20 y CMake >= 3.15.
- Crow 1.3.3 (HTTP/WebSocket), descargado con `FetchContent`.
- nlohmann/json 3.12.0, descargado con `FetchContent`.
- GoogleTest 1.17.0, solo si `BUILD_TESTS=ON`.
- Asio del sistema; los workflows instalan `libasio-dev`.
- La primera configuración necesita red para clonar Crow, json y GoogleTest.

## Comandos

```sh
# Compilar
cmake -S . -B build
cmake --build build --parallel

# Ejecutar (escucha en 0.0.0.0:8080)
./build/agent_monitor

# Compilar y correr pruebas
cmake -S . -B build -DBUILD_TESTS=ON
cmake --build build --parallel
ctest --test-dir build --output-on-failure
```

No hay linter ni formateador configurado. `build/` está en `.gitignore`.

## Mapa del código

- `src/main.cpp`: arranque de Crow, ruta WebSocket `/ws` y `parser()`, que arma
  el JSON de métricas.
- `src/System/System.hpp`: header paraguas que incluye todos los módulos.
- `src/System/CPU/`: `cpu.*` son los modelos; `cpuReader.*` parsea `/proc/stat`
  y `/proc/cpuinfo`, llama a `uname` y calcula el porcentaje entre muestras.
- `src/System/RAM/`: `RAM.*` es el modelo; `RamReader.*` parsea `/proc/meminfo`.
- `src/System/Disk/`: `Disk.*` es el modelo; `DiskReader.*` parsea
  `/proc/mounts`, resuelve dispositivos con sysfs y consulta `statvfs`.
- `src/System/Network/`: `Network.*` es el modelo; `NetworkReader.*` parsea
  `/proc/net/dev`.
- `tests/`: un archivo por módulo; CMake los descubre con GTest.
- `.github/workflows/tests.yaml`: compila y corre pruebas en cada PR.
- `.github/workflows/deploy.yaml`: en `main`, compila, corre pruebas, copia el
  binario por SSH y reinicia un servicio systemd ya existente en el servidor.

## Contrato WebSocket (no romperlo sin avisar)

- URL: `ws://<host>:8080/ws`.
- Al conectar, el servidor manda un texto:
  `El host: <hostname> ha respondido correctamente`.
- Si el cliente manda exactamente `metrics`, recibe un JSON con las claves
  `CPU`, `ram`, `disk` y `red`.
- Cualquier otro mensaje devuelve un texto de error. Los errores también son
  texto plano, no JSON.
- Las claves con errores de ortografía son parte del contrato actual:
  `recived`, `recivedPackets`, `transmitted`, `trasmittedPackets`. No las
  renombres sin actualizar el README (ES/EN) y a los consumidores.
- Tipos y unidades:
  - `CPU.usage`: porcentaje (`double`).
  - `ram.*`: MiB (`float`).
  - `disk.device`: nombre del disco físico, no el punto de montaje.
  - `disk.usage`: bytes totales (`float`); `disk.usageP`: porcentaje usado.
  - `red.<interfaz>.*`: MB (`float`) y paquetes (entero).
- Solo se reporta el disco de `/` (`DiskReader::reader()`).
- `CPU.usage` se calcula contra la muestra anterior. La primera petición tras
  arrancar no tiene muestra y el servidor responde con un texto de error.

## Convenciones

- `#pragma once` en los headers (hoy falta en `DiskReader.hpp`).
- Los nombres de archivo mezclan estilos (`cpu.hpp`, `cpuReader.hpp`,
  `RamReader.hpp`). Sigue el estilo del archivo que toques; no renombres en masa.
- Identificadores, comentarios y mensajes en español. Las claves JSON están en
  inglés y conservan los typos listados arriba.
- Modelos con getters/setters; la lectura de sistema vive en clases `*Reader`
  con métodos estáticos.
- No hay framework de logging: `std::cerr` para errores de arranque.
- Las pruebas leen `/proc` real; no hay mocks ni fixtures. CI corre en Ubuntu.
- No agregues dependencias si la biblioteca estándar alcanza.

## Trampas conocidas

- `parser()` lanza excepción si CPU no tiene muestra. Pasa en la primera
  petición y en dos peticiones dentro del mismo tick de `/proc/stat`.
- El servidor escucha en todas las interfaces y no tiene autenticación.
- El filtro de red solo acepta `enpNsN`, `flannel.N` y `cniN`. En máquinas con
  `eth0`, `ens5` o `wlan0` el objeto `red` queda vacío.
- `CPUReader::readCPU()` vuelve a leer `/proc/cpuinfo` en cada petición.
- `DiskReader::datos` es un estático declarado y definido, pero sin uso.
- El deploy reemplaza el binario en el servidor y reinicia el servicio sin
  respaldo ni rollback. El binario se compila en `ubuntu-latest`.
- El deploy espera estas variables/secretos de GitHub: `SSH_KEY` (secreto) y
  `SSH_SERVER_IP`, `SSH_USER`, `SSH_KNOWN_HOSTS`, `DEPLOY_PATH`,
  `SERVICE_NAME` (variables).

## Antes de cambiar el contrato o los lectores

1. Compila con `-DBUILD_TESTS=ON` y corre `ctest`.
2. Si cambias claves, unidades o mensajes, actualiza README (ES/EN) y pruebas.
3. Si tocas `/proc/mounts` o `/proc/net/dev`, agrega una prueba con una línea de
   ejemplo; hoy esos parsers no tienen cobertura.

## Frontend Rust + Slint (raíz del repo)

En la raíz vive el cliente de escritorio: consume el contrato WebSocket de
arriba y añade una terminal SSH. No sustituye al servidor C++.

```sh
cargo run                 # abre la ventana
cargo test                # parseo de destinos y formato de métricas
```

- `src/main.rs`: arranque, callbacks de Slint y formateo del JSON a la vista.
- `src/admin/conexion.rs`: abre el WS, consume el saludo y pide `metrics` cada
  2 s; reintenta con un contador de generación para cortar tareas viejas.
- `src/admin/ssh.rs`: lanza el binario `ssh` del sistema dentro de una PTY
  (`portable-pty`), interpreta la salida con `vt100` y la muestra como pantalla
  de texto. La contraseña se escribe en la propia terminal.
- `src/admin/system/`: modelos serde del JSON (`CPU`, `ram`, `disk`, `red`).
- `src/ui/`: Slint. `theme.slint` (paleta), `widgets.slint` (panel de vidrio,
  botón, medidor), `connect.slint` y `servstats.slint` (dos columnas: métricas
  y terminal SSH). El tema oscuro de los widgets estándar se fuerza con
  `Palette.color-scheme = ColorScheme.dark`.

Límites conocidos: la rejilla de la terminal es fija (60x30) y no sigue el
tamaño de la ventana; no hay scrollback ni color por celda; el binario necesita
`fontconfig` y un backend gráfico (X11 o Wayland).
