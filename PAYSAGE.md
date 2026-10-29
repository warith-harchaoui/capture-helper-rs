# Paysage

🇫🇷 Français · [🇬🇧 LANDSCAPE.md](https://github.com/warith-harchaoui/capture-helper-rs/blob/master/LANDSCAPE.md)

Crates voisins puis concurrents dans l'espace « capture micro en direct en Rust », comparés à `capture-helper-rs`. Les notes
vont de ⭐ (1) à ⭐⭐⭐⭐⭐ (5), évaluées sur la tâche visée par `capture-helper-rs` :
transformer un micro en direct en un flux de petites trames audio typées (énumérer les périphériques, en ouvrir un, lire des trames de façon bloquante ou non, puis savoir pourquoi un flux s'est arrêté). Un crate optimisé pour un tout autre usage n'est pas pénalisé : la note
reflète seulement l'adéquation à *ce* créneau.

## En un coup d'œil

<!-- TABLE:START -->
| Capture micro en Rust | Énumération des périphériques | API de trames en flux | Normalisation f32 | Contre-pression rendue visible | Erreurs typées | Rééchantillonnage / mixage de canaux | Détection d'activité vocale | Arbre de dépendances léger |
| --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **capture-helper-rs** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐⭐ |
| cpal | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐⭐ |
| libmic-rs | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐ |
| decibri | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ |
| rodio | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐ |
| libpulse-binding | ⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐ | ⭐⭐ | ⭐ | ⭐⭐ |
<!-- TABLE:END -->

## Carte de positionnement

<!-- FIGURE:START -->
Représentation 2D du tableau ci-dessus.

![Carte de positionnement](https://raw.githubusercontent.com/warith-harchaoui/capture-helper-rs/master/assets/paysage.png)

La carte est un résumé en 2D des 8 critères : à lire comme une forme, pas comme un classement. « capture-helper-rs » se situe dans le coin en haut à droite. Les axes se lisent **Horizontal : Précision ↔ Efficacité** et **Vertical : Simplicité ↔ Clarté**.

L'axe horizontal sépare les crates qui restent légers en ne faisant qu'une chose de ceux qui embarquent du DSP : un arbre de dépendances léger pèse positivement vers la droite, tandis que le rééchantillonnage puis la détection d'activité vocale pèsent négativement. L'axe vertical porte sur la reddition de comptes : le crate normalise-t-il ce que le périphérique a donné puis dit-il ce qu'il est advenu de vos trames.

`capture-helper-rs` avec `decibri` partagent le bord supérieur, car tous deux normalisent les échantillons puis rendent compte des trames. Ils occupent les extrémités opposées à l'horizontale et c'est tout le choix : `decibri` apporte le rééchantillonnage avec la VAD, ce crate refuse les deux puis s'en tient à deux dépendances.

Le plus instructif sur cette carte est `cpal`, placé juste sous `capture-helper-rs` à la même abscisse. Même créneau, même légèreté, plus bas sur la reddition de comptes — précisément le manque que ce crate existe pour combler, puisque `cpal` signale les pannes de flux par une fonction de rappel sans chemin de retour vers l'appelant. `libpulse-binding` occupe le bas à gauche comme l'option brute, réservée à Linux.
<!-- FIGURE:END -->

## D'où viennent les affirmations

Versions puis dates de publication lues sur l'API crates.io le 29 octobre 2026 :

| Crate | Version | Dernière publication |
|---|---|---|
| `cpal` | 0.18.2 | 2026-08-16 |
| `libmic-rs` | 0.1.0 | 2025-05-29 |
| `decibri` | 6.4.0 | 2026-10-04 |
| `rodio` | 0.23.0 | 2026-10-11 |

Les notes de `capture-helper-rs` lui-même proviennent de ce dépôt en version 0.1.3.

- **`cpal` n'est pas un concurrent : c'est le socle sur lequel ce crate repose** (`cpal = "0.18"`, la majeure courante). S'il obtient 2 sur la contre-pression puis 3 sur les erreurs typées, ce n'est pas un défaut : il signale les pannes de flux par une fonction de rappel sans chemin de retour vers l'appelant. Tout l'intérêt de `MicCapture::error()` est de rendre cette réponse lisible quelque part.

- **Les deux notes de 1 sur la ligne de ce crate sont réelles puis assumées.** Il n'y a ni rééchantillonnage, ni conversion mono/stéréo, ni détection d'activité vocale : une `MicFrame` porte exactement ce que le périphérique a donné, normalisé en `f32`. `decibri` est le crate à choisir quand le conditionnement avec la VAD sont le besoin.

- **`decibri` obtient 2 sur l'arbre de dépendances** parce que le conditionnement puis la VAD apportent leur propre poids ; ce crate embarque `cpal` avec `thiserror`, `clap` et `hound` restant derrière la fonctionnalité optionnelle `cli`.

Laissé délibérément comme opinion plutôt que sourcé : les colonnes de style
*ergonomie*, puis l'ordre des lignes sous la référence.

## Idées à reprendre

Étudié plutôt que deviné : ce que chaque voisin fait mieux, puis ce que ce crate devrait changer pour l'égaler. Rien ici n'est un engagement : c'est la liste de lecture dont part la version suivante.

- **`decibri` — la détection d'activité vocale, derrière une fonctionnalité** — La capacité manquante la plus nette et l'une des deux notes de 1 de ce crate. `decibri` fait de la VAD sur de l'audio en direct comme enregistré. Tout consommateur d'un flux micro finit par vouloir savoir où se trouve la parole ; l'omettre oblige chacun à en greffer une.

- **`libmic-rs` — enregistrer directement en WAV depuis la bibliothèque** — La CLI écrit déjà un WAV par `hound`. Toute la raison d'être de `libmic-rs` tient dans cet appel et l'exposer depuis la bibliothèque — non seulement depuis le binaire — ne coûterait presque rien.

- **`cpal` — la sélection de périphérique par indice ou sous-chaîne** — Ce crate accepte un nom exact ou le périphérique par défaut, rien d'autre. `cpal` expose l'énumération de l'hôte qui rendrait possible une correspondance par sous-chaîne, puis son abstraction d'hôte désigne aussi les moteurs SoundIO avec PulseAudio.
