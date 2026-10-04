# Chunky for PumpkinMC

An authentic, high-performance chunk pre-generation plugin for **PumpkinMC** (matching Pumpkin `0.2.0+26.3-26.51`), modeled directly after the original [Chunky](https://github.com/pop4959/Chunky) plugin by **pop4959**.

Compiled as a native WebAssembly (`wasm32-wasip2`) component for the Pumpkin server plugin architecture.

---

## Features

- **Chunk Pre-generation**: Fully automated, high-throughput pregeneration that loads and generates terrain chunks outward from spawn or custom coordinates.
- **Spiral Generation Iterator**: Processes chunks in an expanding square/concentric spiral pattern from the center, prioritizing areas nearest to players and spawn.
- **Multiple Shapes**:
  - `square` (default box)
  - `circle` (radial Euclidean disk)
  - `diamond` (Manhattan diamond)
  - `star` (astroid cross curve)
- **World Border Integration**: `/chunky worldborder` automatically detects the world border center and radius from the server.
- **Visible Chunks Per Second (CPS) Counter & Progress**:
  - Live progress reports in console and chat:
    ```
    [Chunky] Task running for world. Processed: 125 chunks (0.03%), ETA: 0:34:28, Rate: 190.0 cps, Current: 13, 6
    ```
- **Task Lifecycle Control**: Full support for starting, pausing, resuming, and cancelling generation tasks (`start`, `pause`, `continue`, `cancel`).
- **Resumable & Persistent**: Automatically preserves progress, selection, and configuration to `plugins/data/Chunky/state.json`. If paused or interrupted, tasks can be resumed seamlessly without re-visiting or skipping chunks.
- **Safe Resource Management**: Drives generation through Pumpkin's native chunk ticket system and unforces chunks after ticket acquisition to prevent unbounded memory growth, while periodically flushing changes to disk with `world.save()`.

---

## Installation

1. Copy [`chunky.wasm`](./chunky.wasm) (or `target/wasm32-wasip2/release/chunky_pumpkin.wasm`) into your Pumpkin server's `plugins/` directory:
   ```bash
   cp chunky.wasm /path/to/pumpkin-server/plugins/
   ```
2. Start or reload your Pumpkin server.
3. Verify that the plugin is loaded using `/plugins`.

---

## Commands

All commands use the root `/chunky` namespace:

| Command | Description |
| :--- | :--- |
| `/chunky` or `/chunky help` | Displays the help menu with all available commands. |
| `/chunky world [world]` | Sets or views the target world (e.g. `world`, `world_nether`, `world_the_end`). If omitted, selects your current world. |
| `/chunky shape <square\|circle\|diamond\|star>` | Sets the generation boundary shape. |
| `/chunky center [<x> <z>]` | Sets the center coordinates in blocks. If omitted, uses your current player coordinates. |
| `/chunky radius <radius>` | Sets the pre-generation radius in blocks (e.g. `1000`). |
| `/chunky worldborder` | Automatically sets the center and radius from the world border. |
| `/chunky selection` | Displays the current selection settings and estimated chunk count. |
| `/chunky start` | Begins pre-generating chunks according to current selection. |
| `/chunky pause` | Pauses the running pre-generation task and saves progress. |
| `/chunky continue` (or `resume`) | Resumes a paused pre-generation task. |
| `/chunky cancel` | Cancels the current task and resets state. |
| `/chunky progress` (or `status`) | Displays immediate progress, remaining chunks, ETA, and CPS rate. |
| `/chunky quiet <seconds>` | Sets the interval for periodic broadcast progress updates (default: `2` seconds). |
| `/chunky reload` | Reloads configuration and saved state from disk. |

---

## Quick Start Example

To pre-generate a 2,000-block radius square area in the overworld:

```text
/chunky world world
/chunky shape square
/chunky center 0 0
/chunky radius 2000
/chunky start
```

To monitor progress:
```text
/chunky progress
```

---

## Building from Source

Requirements:
- Rust 1.85+ (tested with Rust 1.98.1)
- `wasm32-wasip2` target installed:
  ```bash
  rustup target add wasm32-wasip2
  ```

Compile the release WebAssembly component:
```bash
cargo build --release
```

The compiled component is output to:
```text
target/wasm32-wasip2/release/chunky_pumpkin.wasm
```
