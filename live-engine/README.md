# Live engine (Rust)

Remplace les boutons de choix par une conversation dynamique : une IA locale (non censurée selon le modèle
choisi) tient un personnage, parle (voix), t'écoute (micro) et **choisit elle-même** les vidéos/photos de la
médiathèque pour illustrer l'échange. Tout tourne en local, sur ton GPU NVIDIA.

```
App Ubuntu (Electron) ── lance ──> live-engine (Rust, axum, port 3001)
   │  Express :3000 (UI + admin) ── proxy /api/live ──┘   │
   │                                                      ├─ télécharge les modèles (Hugging Face, reprise + SHA-256)
   │                                                      ├─ lit l'architecture réelle du GGUF → plan VRAM
   │                                                      ├─ lance/arrête llama-server  (IA texte, GPU)   :8080
   │                                                      ├─ lance/arrête whisper-server (micro → texte)   :8081
   │                                                      └─ lance/arrête le serveur TTS (voix)            :8880
```

## Utilisation dans l'app

1. **Admin → Modèles IA** (`/admin/live/models`) : clique sur *Télécharger* (Gemma 4 12B Heretic recommandé, puis
   Whisper large-v3-turbo Q5), puis *Charger*. L'écran affiche l'état, le plan mémoire, la VRAM réelle et les journaux.
   Les téléchargements reprennent après une coupure et sont vérifiés par SHA-256. Les derniers modèles choisis se
   rechargent au démarrage.
2. **Admin → Médiathèque** : *Scanner les uploads* (vidéos **et** photos), puis annote tags, ambiance, intensité.
3. **Admin → Personnages** : crée ton personnage (nom, personnalité, style, scénario, voix).
4. **Live** : choisis le personnage, parle ou écris. Parler pendant que l'IA répond la coupe (barge-in).

Ajouter un modèle au catalogue : crée `catalog.json` dans le dossier de données (liste d'objets
`{id, kind: "llm"|"stt"|"tts", name, repo, pattern | files, args, exec}`) ; un même `id` remplace l'entrée intégrée.
`HF_TOKEN` (variable d'environnement) permet de télécharger depuis des dépôts protégés.

## Comment l'IA choisit les médias

Le LLM écrit du texte normal avec des directives invisibles (plus fiable que le function-calling sur modèles locaux) :

| Directive | Effet |
|---|---|
| `[[show: plage, soleil \| mood=joyeux \| intensity=2 \| kind=photo]]` | affiche un média (jamais deux fois le même) |
| `[[ambient: appartement, soir \| mood=calme]]` | change la vidéo d'ambiance en boucle |
| `[[replies: Oui \| Non \| Plus tard]]` | boutons de réponse rapide (optionnels) |
| `[[state: confiance=3]]` | mémoire du récit, réinjectée à chaque tour |

L'IA ne voit que les tags réellement présents dans la médiathèque ; le plafond d'intensité (curseur 1-5) est appliqué
par le serveur.

## Adaptation à 15 Go de VRAM / 64 Go de RAM

Le plan est recalculé à chaque chargement depuis le **vrai** GGUF (couches, têtes KV, dimension de tête, contexte) :
poids + cache KV en q8_0 + tampons, comparés à la VRAM disponible. Si tout tient, `-ngl` met toutes les couches sur le
GPU ; sinon le contexte est réduit d'abord, puis une partie des couches passe en RAM. `cargo run --release plan [id]`
affiche ce plan sans rien lancer. Le calcul du cache KV suppose une attention complète : il surestime les modèles à
fenêtre glissante (Gemma), donc il est prudent.

| Profil | Modèle | Placement |
|---|---|---|
| **A (défaut)** | Gemma 4 12B Heretic Q4_K_M ≈ 7,4 Go, ctx 16k | 100 % GPU |
| **B** | Cydonia 24B IQ4_XS ≈ 12,8 Go | GPU + un peu de RAM |

## Compiler l'app Ubuntu (NVIDIA)

Prérequis : Ubuntu 22.04/24.04, pilote NVIDIA, CUDA Toolkit (`nvcc`), `build-essential cmake git`, Node ≥ 20, Rust.

```bash
npm run dist:linux        # = scripts/build-linux.sh : UI + moteur Rust + llama/whisper-server CUDA + AppImage + .deb
```

Détails (`scripts/build-sidecars.sh`) : `CUDA_ARCH=native` (défaut : la carte de la machine de build ; mets par
exemple `"86;89"` pour un paquet destiné à d'autres cartes), `BUNDLE_CUDA_LIBS=1` embarque `libcudart/libcublas`
(l'AppImage devient autonome mais pèse plus lourd), `LLAMA_CPP_REF` / `WHISPER_CPP_REF` fixent les versions.
Gemma 4 exige un llama.cpp récent (juin 2026 ou après). Les versions compilées sont notées dans `bin/VERSIONS.txt`.

À l'exécution, l'app stocke tout dans ton dossier utilisateur : base et médias dans `~/.config/Virtual Story`,
modèles dans `~/.local/share/virtual-story/models` (`VS_MODELS_DIR` pour changer). Journaux : `~/.config/Virtual
Story/logs/live-engine.log`.

## Développement

```bash
cd live-engine && cargo run --release        # moteur sur :3001 (lit live.toml s'il existe)
cd frontend && npm run dev                   # Vite proxifie /api/live vers :3001
cargo test                                   # planificateur, GGUF, catalogue/téléchargement, directives, médias…
```

## Règles du moteur

- Usage personnel : aucune vérification de médias, aucun refus ni avertissement dans le prompt, curseur d'intensité à 5
  par défaut (réglage à toi).
- Une seule limite est conservée : les personnages sont des adultes. Une fiche avec `age < 18` est refusée à
  l'enregistrement et ignorée au chargement, et le prompt interdit tout contenu sexuel impliquant un mineur.
- Sécurité locale : le moteur écoute sur 127.0.0.1 ; les WebSocket venant d'un autre site web sont refusés ;
  `admin_token` protège l'édition si tu l'exposes sur un réseau.
