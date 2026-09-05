# Capture Helper (Rust)

[🇫🇷](https://github.com/warith-harchaoui/capture-helper-rs/blob/master/LISEZMOI.md) · [🇬🇧](https://github.com/warith-harchaoui/capture-helper-rs/blob/master/README.md)

[![crates.io](https://img.shields.io/crates/v/capture-helper-rs.svg)](https://crates.io/crates/capture-helper-rs) [![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](./LICENSE)

Réécriture en Rust du volet microphone de [`capture-helper`](https://github.com/warith-harchaoui/capture-helper) : transformer un microphone en direct en un flux de petits paquets audio (PCM, l'échantillonnage brut non compressé) exploitables par le reste d'une chaîne de traitement, sans dépendre d'un service tiers.

Ce n'est pas un portage ligne à ligne. Le volet caméra du projet d'origine (`iter_camera_frames`, la GUI de scènes multi-sources, le mixage vidéo) est hors périmètre pour cette version 0.1 : seul le microphone est couvert.

## Ce que fait ce crate (v0.1)

- `list_input_devices() -> Result<Vec<String>, CaptureHelperError>` : énumère les périphériques d'entrée audio du système via [`cpal`](https://crates.io/crates/cpal). Une liste vide est une réponse honnête (aucun micro branché) ; seule une vraie panne d'énumération de l'hôte renvoie une erreur.
- `MicCapture` : ouvre un flux micro (périphérique par défaut ou nommé) et le diffuse comme un itérateur de `MicFrame` — bloquant via `for frame in mic { ... }` / `next_frame()`, ou non bloquant via `try_next_frame()`.
- `MicFrame { samples: Vec<f32>, sample_rate: u32, channels: u16, timestamp: Instant }` : un paquet PCM normalisé en `f32` dans `[-1.0, 1.0]`, quel que soit le format natif du périphérique (`f32`, `i16`, `u16`).
- `CaptureHelperError` (via `thiserror`) : une variante distincte par cause d'échec — aucun périphérique par défaut, nom introuvable, échec d'énumération, échec de lecture de configuration, format d'échantillon non supporté, échec de construction ou de démarrage du flux. Pas de fourre-tout `String`.

## Ce que ce crate ne fait pas (encore, ou pas du tout)

- Pas de caméra, pas d'images, pas de GUI — ce volet du projet d'origine reste là-bas.
- Pas de rééchantillonnage, pas de conversion mono/stéréo, pas de détection d'activité vocale : `MicFrame` livre exactement ce que le périphérique donne, normalisé en `f32`, rien de plus.
- Pas de sélection de périphérique par indice ou sous-chaîne de nom (seulement nom exact ou périphérique par défaut).

## Exemple

```rust
use capture_helper_rs::{list_input_devices, MicCapture};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for name in list_input_devices()? {
        println!("périphérique d'entrée : {name}");
    }

    let mic = MicCapture::from_default_device()?;
    for frame in mic.take(50) {
        println!("{} échantillons @ {} Hz, {} canal(aux)", frame.samples.len(), frame.sample_rate, frame.channels);
    }

    Ok(())
}
```

## Limites de test — à lire avant de faire confiance à la CI

Cet environnement (et probablement le vôtre) n'a pas de microphone réel branché. Concrètement :

- `list_input_devices()` **est** testé : il doit toujours retourner un `Vec` (potentiellement vide) sans jamais paniquer.
- Les chemins d'erreur atteignables sans matériel **sont** testés : demander un périphérique nommé qui n'existe manifestement pas doit produire `CaptureHelperError::DeviceNotFound` (ou `DeviceEnumeration` si l'hôte n'a aucun sous-système audio).
- La capture réelle d'un flux (`MicCapture::from_default_device()` qui reçoit effectivement des échantillons, ou `from_named_device()` sur un nom qui existe vraiment) **n'est pas testée en CI** — ça nécessite un micro physique et une permission OS. Aucun test de ce dépôt ne prétend vérifier ce chemin ; c'est une vérification manuelle à faire sur une machine avec micro avant de s'appuyer dessus en production.

## État du projet

Mesure de couverture faite avec [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) le 2026-08-31, sur macOS (outils LLVM fournis par Xcode) :

| Fichier | Lignes couvertes |
|---|---|
| `src/error.rs` | 100.00% |
| `src/devices.rs` | 92.86% |
| `src/capture.rs` | 15.69% |
| **Total** | **32.03%** (128 lignes, 87 non couvertes) |

Le chiffre global est bas parce que l'essentiel du code non couvert dans `capture.rs` est exactement le chemin décrit ci-dessus (construction et lecture réelle du flux `cpal`) : il ne peut pas être exercé sans microphone physique, et ce crate ne simule pas un faux périphérique juste pour gonfler un pourcentage. `error.rs` et `devices.rs` — le code atteignable sans matériel — sont couverts à 92–100%.

Pour reproduire :

```bash
cargo install cargo-llvm-cov
# macOS sans rustup : pointer sur les outils LLVM d'Xcode
export LLVM_COV=$(xcrun --find llvm-cov)
export LLVM_PROFDATA=$(xcrun --find llvm-profdata)
# avec rustup (Linux/Windows/macOS) : rustup component add llvm-tools-preview

cargo llvm-cov --summary-only
```

## Installation

```toml
[dependencies]
capture-helper-rs = "0.1"
```

Prérequis : Rust stable récent. `cpal` gère nativement CoreAudio (macOS), WASAPI (Windows) et ALSA/PulseAudio/JACK/PipeWire (Linux) — pas de dépendance externe type `ffmpeg` ou `PortAudio` à installer séparément.

## Vérifications avant de pousser

```bash
scripts/install-hooks.sh   # une fois par clone : installe le garde-fou pre-push ci-dessous
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all
scripts/check-fresh-resolve.sh   # compile sans Cargo.lock, comme un consommateur en aval
```

Le dernier existe parce que les trois autres, et la CI, compilent tous contre le
`Cargo.lock` commité — que personne n'utilise en aval du crate publié. Un intervalle de
dépendance devenu mauvais reste vert ici tout en cassant chaque `cargo add` / `cargo
install` neuf, et `cargo publish --dry-run` ne le voit pas non plus (il vérifie avec le
même lock). La CI lance ce contrôle une fois, sous Linux.

## Projets liés

Fait partie du même socle d'outils locaux que [`capture-helper`](https://github.com/warith-harchaoui/capture-helper) (Python) et la suite [AI Helpers](https://github.com/warith-harchaoui/ai-helpers). Réécriture indépendante, pas une liaison (*binding*).

## Auteur

- [Warith HARCHAOUI](https://linkedin.com/in/warith-harchaoui)

## Licence

BSD-3-Clause — voir [LICENSE](./LICENSE).
